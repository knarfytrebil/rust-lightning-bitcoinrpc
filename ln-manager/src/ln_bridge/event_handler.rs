use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Instant};

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
use crate::utils::{compact_btc_to_bech32};
use log::{info};

async fn handle_fund_tx<T: Larva>(
    mut self_sender: mpsc::Sender<()>,
    &temporary_channel_id: &[u8; 32],
    this: Arc<EventHandler<T>>,
    value: &[&str; 2]
) {
    let tx_hex = this.rpc_client.make_rpc_call(
        "createrawtransaction",
        value,
        false
    ).await.unwrap();

    let funded_tx_args = &[&format!("\"{}\"", tx_hex.as_str().unwrap())[..]];
    let funded_tx = this.rpc_client.make_rpc_call(
        "fundrawtransaction",
        funded_tx_args,
        false
    ).await.unwrap();
    
    info!("funded_tx: {}", &funded_tx);
    let changepos = funded_tx["changepos"].as_i64().unwrap();
    info!("change pos: {}", &changepos);
    assert!(changepos == 0 || changepos == 1);

    let signed_tx_args = &[&format!("\"{}\"", funded_tx["hex"].as_str().unwrap())[..]];
    let signed_tx = this.rpc_client.make_rpc_call(
        "signrawtransactionwithwallet",
        signed_tx_args,
        false
    ).await.unwrap();

    assert_eq!(signed_tx["complete"].as_bool().unwrap(), true);
    let tx: blockdata::transaction::Transaction = encode::deserialize(&hex_to_vec(&signed_tx["hex"].as_str().unwrap()).unwrap()).unwrap();
    let outpoint = chain::transaction::OutPoint {
        txid: tx.txid(),
        index: if changepos == 0 { 1 } else { 0 },
    };
    this.channel_manager.funding_transaction_generated(&temporary_channel_id, outpoint);
    this.txn_to_broadcast.lock().unwrap().insert(outpoint, tx);
    let _ = self_sender.try_send(());
    info!("Generated funding tx!");
}

async fn handle_events<T: Larva>(
    this: &Arc<EventHandler<T>>,
    self_sender: &mpsc::Sender<()>,
    larva: &impl Larva,
) {
    this.peer_manager.process_events();
    let mut events = this.channel_manager.get_and_clear_pending_events();
    events.append(&mut this.monitor.get_and_clear_pending_events());
    for event in events {
        match event {
            Event::FundingGenerationReady { temporary_channel_id, channel_value_satoshis, output_script, .. } => {
                let bech_32_network = compact_btc_to_bech32(this.network);
                let addr = bitcoin_bech32::WitnessProgram::from_scriptpubkey(&output_script[..], bech_32_network)
                    .expect("LN funding tx should always be to a SegWit output").to_address();
                let handle_fund_tx_args = &["[]", &format!("{{\"{}\": {}}}", addr, channel_value_satoshis as f64 / 1_000_000_00.0)];
                let _ = handle_fund_tx(
                    self_sender.clone(),
                    &temporary_channel_id,
                    this.clone(),
                    handle_fund_tx_args 
                ).await;
            },
            Event::PaymentReceived { payment_hash, amt } => {
                let images = this.payment_preimages.lock().unwrap();
                if let Some(payment_preimage) = images.get(&payment_hash) {
                    if this.channel_manager.claim_funds(payment_preimage.clone()) {
                        info!("Payment received: {} msat id {}", amt, hex_str(&payment_hash.0));
                    } else {
                        info!("Failed to claim money we were told we had?");
                    }
                } else {
                    this.channel_manager.fail_htlc_backwards(&payment_hash);
                    info!("Received payment but we didn't know the preimage :(");
                }
                let mut sender = self_sender.clone();
                let _ = sender.try_send(());
            },
            Event::PendingHTLCsForwardable { time_forwardable } => {
                let this = this.clone();
                let mut sender = self_sender.clone();
                let deadline = Instant::now().checked_add(time_forwardable).unwrap();
                let _ = larva.spawn_task(Box::new(tokio::timer::delay(deadline).then(move |_| {
                    this.channel_manager.process_pending_htlc_forwards();
                    let _ = sender.try_send(());
                    future::ok(())
                })));
            },
            Event::FundingBroadcastSafe { funding_txo, .. } => {
                let mut txn = this.txn_to_broadcast.lock().unwrap();
                let tx = txn.remove(&funding_txo).unwrap();
                this.broadcaster.broadcast_transaction(&tx);
                info!("Broadcast funding tx {}!", tx.txid());
            },
            Event::PaymentSent { payment_preimage } => {
                info!("Payment Sent, proof: {}", hex_str(&payment_preimage.0));
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

    let filename = format!("{}/manager_data", this.file_prefix);
    let tmp_filename = filename.clone() + ".tmp";

    {
        let mut f = fs::File::create(&tmp_filename).unwrap();
        this.channel_manager.write(&mut f).unwrap();
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
