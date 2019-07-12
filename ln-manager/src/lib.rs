extern crate base64;
extern crate bitcoin;
extern crate bitcoin_bech32;
extern crate bitcoin_hashes;
extern crate bytes;
extern crate config;
extern crate exit_future;
extern crate futures;
extern crate hyper;
extern crate lightning;
extern crate lightning_invoice;
extern crate lightning_net_tokio;
extern crate log;
extern crate num_traits;
extern crate rand;
extern crate secp256k1;
extern crate serde_json;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_fs;
extern crate tokio_io;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate num_derive;

mod chain_monitor;
mod channel_monitor;
mod event_handler;
pub mod executor;
pub mod ln_bridge;
mod rpc_client;

use std::mem;

use std::collections::HashMap;
use std::fs;
// use std::mem;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
// use std::vec::Vec;

// use substrate_service::{SpawnTaskHandle, Executor};
use exit_future::Exit;
use futures::future;
use futures::future::Executor;

use futures::future::Future;
use futures::sync::mpsc;
use futures::Stream;

use bitcoin::network::constants;
use bitcoin::util::address::Address;
use bitcoin::util::bip32;
use lightning::chain::keysinterface::{KeysInterface, KeysManager};
use lightning::ln::channelmanager::{ChannelManager, PaymentHash, PaymentPreimage};
use lightning::ln::peer_handler::PeerManager;
use lightning::ln::{channelmanager, channelmonitor, peer_handler, router};
use lightning_net_tokio::{Connection, SocketDescriptor};
use secp256k1::key::PublicKey;
use secp256k1::{All, Secp256k1};

use chain_monitor::{spawn_chain_monitor, ChainInterface, FeeEstimator};
use channel_monitor::ChannelMonitor;
use event_handler::EventHandler;
use rpc_client::RPCClient;

use ln_bridge::channel_manager::RestoreArgs as RestoreManagerArgs;
use ln_bridge::log_printer::LogPrinter;
use ln_bridge::settings::Settings;
use ln_bridge::Restorable;
use log::{error, info};

use executor::Larva;

pub struct LnManager {
    pub rpc_client: Arc<RPCClient>,
    pub network: constants::Network,
    pub router: Arc<router::Router>,
    pub event_notify: mpsc::Sender<()>,
    pub channel_manager: Arc<ChannelManager>,
    pub peer_manager: Arc<PeerManager<SocketDescriptor>>,
    pub payment_preimages: Arc<Mutex<HashMap<PaymentHash, PaymentPreimage>>>,
    pub secp_ctx: Secp256k1<All>,
    pub keys: Arc<KeysManager>,
    pub settings: Settings,
}

impl LnManager {
    pub fn new(settings: Settings, larva: impl Larva, exit: Exit) -> Self {
        let logger = Arc::new(LogPrinter {});
        let rpc_client = Arc::new(RPCClient::new(settings.rpc_url.clone()));
        let secp_ctx = Secp256k1::new();
        let fee_estimator = Arc::new(FeeEstimator::new());

        info!("Checking validity of RPC URL to bitcoind...");
        let network =
            LnManager::get_network(&rpc_client, &larva, exit.clone()).unwrap();

        info!("Success! Starting up...");
        if network == constants::Network::Bitcoin {
            panic!("LOL, you're insane");
        }

        let data_path = settings.lndata.clone();
        if !fs::metadata(&data_path).unwrap().is_dir() {
            panic!("Need storage_directory_path to exist and be a directory (or symlink to one)");
        }
        let _ = fs::create_dir(data_path.clone() + "/monitors"); // If it already exists, ignore, hopefully perms are ok

        let our_node_seed = ln_bridge::key::get_key_seed(data_path.clone());
        let keys = Arc::new(KeysManager::new(&our_node_seed, network, logger.clone()));
        let (import_key_1, import_key_2) =
            bip32::ExtendedPrivKey::new_master(network, &our_node_seed)
                .map(|extpriv| {
                    (
                        extpriv
                            .ckd_priv(&secp_ctx, bip32::ChildNumber::from_hardened_idx(1).unwrap())
                            .unwrap()
                            .private_key
                            .key,
                        extpriv
                            .ckd_priv(&secp_ctx, bip32::ChildNumber::from_hardened_idx(2).unwrap())
                            .unwrap()
                            .private_key
                            .key,
                    )
                })
                .unwrap();

        /* ==> For debug */
        let pub_key_1 = bitcoin::util::key::PrivateKey {
            key: import_key_1,
            compressed: true,
            network,
        }
        .public_key(&secp_ctx);
        let pub_key_2 = bitcoin::util::key::PrivateKey {
            key: import_key_2,
            compressed: true,
            network,
        }
        .public_key(&secp_ctx);

        println!(
            "Address - ChannelMonitor Claim: {:?}",
            &Address::p2pkh(&pub_key_1, constants::Network::Regtest)
        );
        println!(
            "Address - Cooperative Close: {:?}",
            &Address::p2pkh(&pub_key_2, constants::Network::Regtest)
        );
        /* <== For debug */

        // let (import_key_1, import_key_2) = ln_bridge::key::extprivkey(network, &our_node_seed, &secp_ctx);
        let chain_monitor = Arc::new(ChainInterface::new(
            rpc_client.clone(),
            network,
            logger.clone(),
            larva.clone(),
            exit.clone(),
        ));

        let _ = larva.clone().spawn_task(
            rpc_client
                .make_rpc_call(
                    "importprivkey",
                    &[
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
                    ],
                    false,
                )
                .then(|_| Ok(()))
                .select(exit.clone())
                .then(|_| Ok(())),
        );

        let _ = larva.clone().spawn_task(
            rpc_client
                .make_rpc_call(
                    "importprivkey",
                    &[
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
                    ],
                    false,
                )
                .then(|_| Ok(()))
                .select(exit.clone())
                .then(|_| Ok(())),
        );

        let monitors_loaded = ChannelMonitor::load_from_disk(&(data_path.clone() + "/monitors"));

        let monitor = Arc::new(ChannelMonitor {
            monitor: channelmonitor::SimpleManyChannelMonitor::new(
                chain_monitor.clone(),
                chain_monitor.clone(),
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
            chain_monitor.clone(), // chain watcher
            chain_monitor.clone(), // chain broadcaster
            logger.clone(),
            keys.clone(),
        ));

        let router = Arc::new(router::Router::new(
            PublicKey::from_secret_key(&secp_ctx, &keys.get_node_secret()),
            chain_monitor.clone(), // chain watch
            logger.clone(),
        ));

        let peer_manager = Arc::new(peer_handler::PeerManager::new(
            peer_handler::MessageHandler {
                chan_handler: channel_manager.clone(),
                route_handler: router.clone(),
            },
            keys.get_node_secret(),
            logger.clone(),
        ));

        let payment_preimages = Arc::new(Mutex::new(HashMap::new()));

        let event_notify = EventHandler::setup(
            network,
            data_path,
            rpc_client.clone(),
            peer_manager.clone(),
            monitor.monitor.clone(),
            channel_manager.clone(),
            chain_monitor.clone(), // chain broadcaster
            payment_preimages.clone(),
            larva.clone(),
            exit.clone(),
        );

        let listener =
            tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", settings.port).parse().unwrap())
                .unwrap();

        let peer_manager_listener = peer_manager.clone();
        let event_listener = event_notify.clone();
        let _ = larva.spawn_task(
            listener
                .incoming()
                .for_each(move |sock| {
                    info!("Got new inbound connection, waiting on them to start handshake...");
                    Connection::setup_inbound(
                        peer_manager_listener.clone(),
                        event_listener.clone(),
                        sock,
                    );
                    Ok(())
                })
                .then(|_| Ok(())),
        );

        spawn_chain_monitor(
            fee_estimator,
            rpc_client.clone(),
            chain_monitor,
            event_notify.clone(),
            larva.clone(),
            exit.clone(),
        );

        let _ = larva.clone().spawn_task(Box::new(
            tokio::timer::Interval::new(Instant::now(), Duration::new(1, 0))
                .for_each(move |_| {
                    //TODO: Regularly poll chain_monitor.txn_to_broadcast and send them out
                    Ok(())
                })
                .map_err(|_| ())
                .select(exit.clone())
                .then(|_| Ok(())),
        ));

        Self {
            rpc_client,
            //
            network,
            router,
            event_notify,
            channel_manager,
            peer_manager,
            payment_preimages,
            secp_ctx,
            keys,
            settings,
        }
    }

    pub fn get_network(
        rpc_client: &Arc<RPCClient>,
        larva: &impl Larva,
        exit: Exit,
    ) -> Result<constants::Network, &'static str> {
        let thread_rt = tokio::runtime::current_thread::Runtime::new().unwrap();
        // Blocked Here
        // thread_rt.block_on(
        //     rpc_client
        //     .make_rpc_call("getblockchaininfo", &[], false)
        //     .and_then(|v| {
        //         println!("{:?}", &v);
        //         assert!(v["verificationprogress"].as_f64().unwrap() > 0.99);
        //         assert_eq!(v["bip9_softforks"]["segwit"]["status"].as_str().unwrap(), "active");
        //
        //         Ok(Ok(constants::Network::Testnet))
        //         // match v["chain"].as_str().unwrap() {
        //         //     "main" => Ok(constants::Network::Bitcoin),
        //         //     "test" => Ok(constants::Network::Testnet),
        //         //     "regtest" => Ok(constants::Network::Regtest),
        //         //     _ => panic!("Unknown Network"),
        //         // }
        //         // Ok(())
        //     })
        // ).unwrap()
        Ok(constants::Network::Regtest)
    }
}
