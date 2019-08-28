#![feature(async_await)]
#![feature(async_closure)]
extern crate base64;
extern crate bitcoin;
extern crate bitcoin_bech32;
extern crate bitcoin_hashes;
extern crate bytes;
extern crate config;
extern crate futures;
extern crate hyper;
extern crate lightning;
extern crate lightning_invoice;
extern crate num_traits;
extern crate rand;
extern crate secp256k1;
extern crate serde_json;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_fs;
extern crate tokio_io;
extern crate tokio_tcp;
extern crate tokio_timer;
extern crate futures_timer;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

pub mod executor;
pub mod ln_bridge;
pub mod ln_cmd;
pub mod utils;

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;
use futures::future;
use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};

use bitcoin::network::constants;
use lightning::chain::keysinterface::{KeysInterface, KeysManager};
use lightning::ln::channelmanager::{ChannelManager, PaymentHash, PaymentPreimage};
use lightning::ln::peer_handler::PeerManager;
use lightning::ln::{channelmanager, channelmonitor, peer_handler, router};
use lightning::util::logger::{Level};
use secp256k1::key::PublicKey;
use secp256k1::{All, Secp256k1};

use ln_bridge::connection::{Connection, SocketDescriptor};
use ln_bridge::chain_monitor::{spawn_chain_monitor, ChainWatchInterfaceUtil, ChainBroadcaster, FeeEstimator};
use ln_bridge::channel_monitor::ChannelMonitor;
use ln_bridge::channel_manager::RestoreArgs as RestoreManagerArgs;
use ln_bridge::event_handler::EventHandler;
use ln_bridge::rpc_client::RPCClient;
use ln_bridge::log_printer::LogPrinter;
use ln_bridge::settings::Settings;
use ln_bridge::Restorable;

use executor::Larva;

pub struct LnManager<T: Larva> {
    pub rpc_client: Arc<RPCClient>,
    pub network: constants::Network,
    pub router: Arc<router::Router>,
    pub event_notify: mpsc::Sender<()>,
    pub channel_manager: Arc<ChannelManager>,
    pub peer_manager: Arc<PeerManager<SocketDescriptor<T>>>,
    pub payment_preimages: Arc<Mutex<HashMap<PaymentHash, PaymentPreimage>>>,
    pub secp_ctx: Secp256k1<All>,
    pub keys: Arc<KeysManager>,
    pub settings: Settings,
    pub larva: T,
}

impl_command!(LnManager);

impl<T: Larva> LnManager<T> {
    pub async fn new(settings: Settings, larva: T) -> Result<Self, ()> {

        // Logger
        let logger = Arc::new(LogPrinter { level: Level::Debug });
        let rpc_client = Arc::new(RPCClient::new(settings.bitcoind.rpc_url.clone()));
        let secp_ctx = Secp256k1::new();
        let fee_estimator = Arc::new(FeeEstimator::new());

        info!("Checking validity of RPC URL to bitcoind...");
        let network = get_network(&rpc_client).await?;
        info!("Success! Starting up...");

        // Data Storage
        let data_path = settings.lightning.lndata.clone();
        if !fs::metadata(&data_path).unwrap().is_dir() {
            panic!("Need storage_directory_path to exist and be a directory (or symlink to one)");
        }

        let _ = fs::create_dir(data_path.clone() + "/monitors"); // If it already exists, ignore, hopefully perms are ok

        // Key Seed
        let our_node_seed = ln_bridge::key::get_key_seed(data_path.clone());

        let (secs, nano) = get_seeds_from_time();
        let keys = Arc::new(KeysManager::new(&our_node_seed, network, logger.clone(), secs, nano));

        let (import_key_1, import_key_2) = ln_bridge::key::get_import_secret_keys(network, &our_node_seed);

        let chain_watcher = Arc::new(ChainWatchInterfaceUtil::new(network, logger.clone()));
        let chain_broadcaster = Arc::new(ChainBroadcaster::new(rpc_client.clone(),larva.clone()));

        let async_client = rpc_client.clone();
        let _ = larva.clone().spawn_task(async move {
            let k = &[
                &("\"".to_string()
                  + &bitcoin::util::key::PrivateKey {
                      key: import_key_1,
                      compressed: true,
                      network,
                  }
                  .to_wif()
                  + "\""),
                "\"rust-lightning ChannelMonitor claim\"",
                "false",
            ];
            async_client.make_rpc_call("importprivkey", k, false).map(|_| Ok(())).await
        });
        let async_client = rpc_client.clone();
        let _ = larva.clone().spawn_task(async move {
            let k = &[
                &("\"".to_string()
                  + &bitcoin::util::key::PrivateKey {
                      key: import_key_2,
                      compressed: true,
                      network,
                  }
                  .to_wif()
                  + "\""),
                "\"rust-lightning cooperative close\"",
                "false",
            ];
            async_client.make_rpc_call("importprivkey", k, false).map(|_| Ok(())).await
        });

        let monitors_loaded = ChannelMonitor::load_from_disk(&(data_path.clone() + "/monitors"));

        let monitor = Arc::new(ChannelMonitor {
            monitor: channelmonitor::SimpleManyChannelMonitor::new(
                chain_watcher.clone(),
                chain_broadcaster.clone(),
                logger.clone(),
                fee_estimator.clone(),
            ),
            file_prefix: data_path.clone() + "/monitors",
        });

        let channel_manager = channelmanager::ChannelManager::try_restore(RestoreManagerArgs::new(
            data_path.clone(),
            monitors_loaded,
            network.clone(),
            fee_estimator.clone(),
            monitor.clone(),
            chain_watcher.clone(),
            chain_broadcaster.clone(),
            logger.clone(),
            keys.clone(),
        ));

        let router = Arc::new(router::Router::new(
            PublicKey::from_secret_key(&secp_ctx, &keys.get_node_secret()),
            chain_watcher.clone(), // chain watch
            logger.clone(),
        ));

        let peer_manager = Arc::new(peer_handler::PeerManager::new(
            peer_handler::MessageHandler {
                chan_handler: channel_manager.clone(),
                route_handler: router.clone(),
            },
            keys.get_node_secret(),
            &rand::thread_rng().gen::<[u8; 32]>(),
            logger.clone(),
        ));

        let payment_preimages = Arc::new(Mutex::new(HashMap::new()));

        // clone for move (handle receiver)
        let event_notify = EventHandler::<T>::setup(
            network,
            data_path,
            rpc_client.clone(),
            peer_manager.clone(),
            monitor.monitor.clone(),
            channel_manager.clone(),
            chain_broadcaster.clone(), // chain broadcaster
            payment_preimages.clone(),
            larva.clone(),
        );

        let peer_manager_listener = peer_manager.clone();
        let event_listener = event_notify.clone();

        info!("Lightning Port binded on 0.0.0.0:{}", &settings.lightning.port);
        let setup_larva = larva.clone();
        let listener =
            tokio_tcp::TcpListener::bind(&format!("0.0.0.0:{}", settings.lightning.port).parse().unwrap())
            .unwrap();

        let _ = larva.clone().spawn_task(
            listener
                .incoming()
                .for_each(move |sock| {
                    info!("Got new inbound connection, waiting on them to start handshake...");
                    Connection::setup_inbound(
                        peer_manager_listener.clone(),
                        event_listener.clone(),
                        sock.unwrap(),
                        setup_larva.clone(),
                    );
                    future::ready(())
                })
                .map(|_| Ok(())),
        );

        let _ = larva.clone().spawn_task(
            async {
                spawn_chain_monitor(
                    fee_estimator,
                    rpc_client.clone(),
                    chain_watcher,
                    chain_broadcaster,
                    event_notify.clone(),
                    larva.clone(),
                ).map(| _| Ok(()))
            }.await
        );

        // TODO see below
        // let _ = larva.clone().spawn_task(Box::new(
        //     tokio::timer::Interval::new(Instant::now(), Duration::new(1, 0))
        //         .for_each(move |_| {
        //             //TODO: Regularly poll chain_monitor.txn_to_broadcast and send them out
        //
        //             future::ready(())
        //         })
        //         .map_err(|_| ())
        // ));

        let ln_manager = Self {
            rpc_client,
            network,
            router,
            event_notify,
            channel_manager,
            peer_manager,
            payment_preimages,
            secp_ctx,
            keys,
            settings,
            larva,
        };

        Ok(ln_manager)
    }

}

fn get_seeds_from_time() -> (u64, u32) {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    (since_the_epoch.as_secs() as u64, since_the_epoch.subsec_nanos() as u32)
}

pub async fn get_network(
    rpc_client: &Arc<RPCClient>,
) -> Result<constants::Network, ()> {
    let v = rpc_client.make_rpc_call("getblockchaininfo", &[], false).await?;
    assert!(v["verificationprogress"].as_f64().unwrap() > 0.99);
    assert_eq!(v["bip9_softforks"]["segwit"]["status"].as_str().unwrap(), "active");
    match v["chain"].as_str().unwrap() {
        "main" => { 
            panic!("LOL, you're insane");
            // Ok(constants::Network::Bitcoin) 
        },
        "test" => Ok(constants::Network::Testnet),
        "regtest" => Ok(constants::Network::Regtest),
        _ => panic!("Unknown Network")
    }
}
