use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};

use crate::future;
use futures::channel::mpsc;
use futures::{StreamExt, FutureExt};

use bitcoin::blockdata;
use bitcoin::consensus::encode;
use bitcoin::network::constants;

use lightning::chain;
use lightning::chain::keysinterface::SpendableOutputDescriptor;
use lightning::ln::channelmanager;
use lightning::ln::channelmanager::{PaymentHash, PaymentPreimage};
use lightning::ln::channelmonitor;
use lightning::ln::peer_handler;
use lightning::util::events::{Event, EventsProvider};
use lightning::util::ser::Writeable; 
use super::connection::SocketDescriptor;

use super::utils::{hex_to_vec, hex_str};
use super::rpc_client::RPCClient;
use crate::executor::Larva;
use log::{info};

async fn handle_fund_tx<T: Larva>(
    mut self_sender: mpsc::Sender<()>,
    &temporary_channel_id: &[u8; 32],
    us: Arc<EventHandler<T>>,
    value: &[&str; 2]
) {
    let tx_hex = us.rpc_client.make_rpc_call(
        "createrawtransaction",
        value,
        false
    ).await.unwrap();

    let funded_tx_args = &[&format!("\"{}\"", tx_hex.as_str().unwrap())[..]];
    let funded_tx = us.rpc_client.make_rpc_call(
        "fundrawtransaction",
        funded_tx_args,
        false
    ).await.unwrap();
    
    let changepos = funded_tx["changepos"].as_i64().unwrap();
    assert!(changepos == 0 || changepos == 1);

    let signed_tx_args = &[&format!("\"{}\"", funded_tx["hex"].as_str().unwrap())[..]];
    let signed_tx = us.rpc_client.sync_rpc_call(
        "signrawtransactionwithwallet",
        signed_tx_args,
        false
    ).unwrap();

    assert_eq!(signed_tx["complete"].as_bool().unwrap(), true);
    let tx: blockdata::transaction::Transaction = encode::deserialize(&hex_to_vec(&signed_tx["hex"].as_str().unwrap()).unwrap()).unwrap();
    let outpoint = chain::transaction::OutPoint {
        txid: tx.txid(),
        index: if changepos == 0 { 1 } else { 0 },
    };
    us.channel_manager.funding_transaction_generated(&temporary_channel_id, outpoint);
    us.txn_to_broadcast.lock().unwrap().insert(outpoint, tx);
    warn!(">>> SEND FROM HANDLE FUND TX");
    let _ = self_sender.try_send(());
    info!("Generated funding tx!");
}

async fn handle_events<T: Larva>(
    us: &Arc<EventHandler<T>>,
    self_sender: &mpsc::Sender<()>,
    larva: &impl Larva,
) {
    us.peer_manager.process_events();
    let mut events = us.channel_manager.get_and_clear_pending_events();
    events.append(&mut us.monitor.get_and_clear_pending_events());
    for event in events {
        match event {
            Event::FundingGenerationReady { temporary_channel_id, channel_value_satoshis, output_script, .. } => {
                let addr = bitcoin_bech32::WitnessProgram::from_scriptpubkey(&output_script[..], match us.network {
                    constants::Network::Bitcoin => bitcoin_bech32::constants::Network::Bitcoin,
                    constants::Network::Testnet => bitcoin_bech32::constants::Network::Testnet,
                    constants::Network::Regtest => bitcoin_bech32::constants::Network::Regtest,
                }).expect("LN funding tx should always be to a SegWit output").to_address();
                let handle_fund_tx_args = &["[]", &format!("{{\"{}\": {}}}", addr, channel_value_satoshis as f64 / 1_000_000_00.0)];
                let _ = handle_fund_tx(
                    self_sender.clone(),
                    &temporary_channel_id,
                    us.clone(),
                    handle_fund_tx_args 
                ).await;
            },
            Event::PaymentReceived { payment_hash, amt } => {
                let images = us.payment_preimages.lock().unwrap();
                if let Some(payment_preimage) = images.get(&payment_hash) {
                    if us.channel_manager.claim_funds(payment_preimage.clone()) {
                        info!("Moneymoney! {} id {}", amt, hex_str(&payment_hash.0));
                    } else {
                        info!("Failed to claim money we were told we had?");
                    }
                } else {
                    us.channel_manager.fail_htlc_backwards(&payment_hash);
                    info!("Received payment but we didn't know the preimage :(");
                }
                let mut sender = self_sender.clone();
                warn!(">>> SEND FROM PAYMENT RECEIVED");
                let _ = sender.try_send(());
            },
            Event::PendingHTLCsForwardable { time_forwardable } => {
                let us = us.clone();
                let mut sender = self_sender.clone();
                let _ = larva.spawn_task(Box::new(tokio::timer::Delay::new(time_forwardable).then(move |_| {
                    us.channel_manager.process_pending_htlc_forwards();
                    let _ = sender.try_send(());
                    warn!(">>> SEND FROM PENDING HTLC");
                    future::ok(())
                })));
            },
            Event::FundingBroadcastSafe { funding_txo, .. } => {
                let mut txn = us.txn_to_broadcast.lock().unwrap();
                let tx = txn.remove(&funding_txo).unwrap();
                us.broadcaster.broadcast_transaction(&tx);
                info!("Broadcast funding tx {}!", tx.txid());
            },
            Event::PaymentSent { payment_preimage } => {
                info!("Less money :(, proof: {}", hex_str(&payment_preimage.0));
            },
            Event::PaymentFailed { payment_hash, rejected_by_dest } => {
                info!("{} failed id {}!", if rejected_by_dest { "Send" } else { "Route" }, hex_str(&payment_hash.0));
            },
            Event::SpendableOutputs { mut outputs } => {
                for output in outputs.drain(..) {
                    match output {
                        SpendableOutputDescriptor:: StaticOutput { outpoint, .. } => {
                            info!("Got on-chain output Bitcoin Core should know how to claim at {}:{}", hex_str(&outpoint.txid[..]), outpoint.vout);
                        },
                        SpendableOutputDescriptor::DynamicOutputP2WSH { .. } => {
                            info!("Got on-chain output we should claim...");
                            //TODO: Send back to Bitcoin Core!
                        },
                        SpendableOutputDescriptor::DynamicOutputP2WPKH { .. } => {
                            info!("Got on-chain output we should claim...");
                            //TODO: Send back to Bitcoin Core!
                        },
                    }
                }
            }
        }
    }

    let filename = format!("{}/manager_data", us.file_prefix);
    let tmp_filename = filename.clone() + ".tmp";

    {
        let mut f = fs::File::create(&tmp_filename).unwrap();
        us.channel_manager.write(&mut f).unwrap();
    }
    fs::rename(&tmp_filename, &filename).unwrap();
}

pub struct EventHandler<T: Larva> {
    network: constants::Network,
    file_prefix: String,
    rpc_client: Arc<RPCClient>,
    peer_manager: Arc<peer_handler::PeerManager<SocketDescriptor<T>>>,
    channel_manager: Arc<channelmanager::ChannelManager>,
    monitor: Arc<channelmonitor::SimpleManyChannelMonitor<chain::transaction::OutPoint>>,
    broadcaster: Arc<dyn chain::chaininterface::BroadcasterInterface>,
    txn_to_broadcast: Mutex<HashMap<chain::transaction::OutPoint, blockdata::transaction::Transaction>>,
    payment_preimages: Arc<Mutex<HashMap<PaymentHash, PaymentPreimage>>>,
}

impl<T: Larva> EventHandler<T> {
    pub fn setup(
        network: constants::Network,
        file_prefix: String,
        rpc_client: Arc<RPCClient>,
        peer_manager: Arc<peer_handler::PeerManager<SocketDescriptor<T>>>,
        monitor: Arc<channelmonitor::SimpleManyChannelMonitor<chain::transaction::OutPoint>>,
        channel_manager: Arc<channelmanager::ChannelManager>,
        broadcaster: Arc<dyn chain::chaininterface::BroadcasterInterface>,
        payment_preimages: Arc<Mutex<HashMap<PaymentHash, PaymentPreimage>>>,
        larva: impl Larva,
    ) -> mpsc::Sender<()> {
        let this = Arc::new(Self {
            network,
            file_prefix,
            rpc_client,
            peer_manager,
            channel_manager,
            monitor,
            broadcaster,
            txn_to_broadcast: Mutex::new(HashMap::new()),
            payment_preimages,
        });
        let (sender, receiver) = mpsc::channel(2);
        let self_sender = sender.clone();

        let _ = larva.clone().spawn_task(
            async move {
                receiver.for_each(|_| { handle_events(&this, &self_sender, &larva) }).await;
                Ok(())
            }
        );
        sender
    }
}
