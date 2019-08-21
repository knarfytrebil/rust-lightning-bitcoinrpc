use super::rpc_client::{GetHeaderResponse, RPCClient};
use super::utils::hex_to_vec;

use tokio::runtime::current_thread::Runtime;
use std::thread;

use bitcoin;
use serde_json;
// use tokio_timer;

use bitcoin_hashes::hex::ToHex;
use bitcoin_hashes::sha256d::Hash as Sha256dHash;

use futures::future;
use futures::prelude::*;
use futures::future::Future;
use futures::channel::mpsc;
use futures::{StreamExt};
use futures::executor::block_on;
use futures_timer::Interval;

use lightning::chain::chaininterface;
pub use lightning::chain::chaininterface::{ChainWatchInterface, ChainWatchInterfaceUtil};

use bitcoin::blockdata::block::Block;
use bitcoin::consensus::encode;
use bitcoin::util::hash::BitcoinHash;

use crate::executor::Larva;
use log::info;
use std::cmp;
use std::collections::HashMap;
use std::marker::{Sync};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::time::{Duration};
use std::vec::Vec;

pub struct FeeEstimator {
    background_est: AtomicUsize,
    normal_est: AtomicUsize,
    high_prio_est: AtomicUsize,
}

impl FeeEstimator {
    pub fn new() -> Self {
        FeeEstimator {
            background_est: AtomicUsize::new(0),
            normal_est: AtomicUsize::new(0),
            high_prio_est: AtomicUsize::new(0),
        }
    }

    pub async fn update_values(this: Arc<Self>, rpc_client: Arc<RPCClient>) -> Result<(), ()> {
        // Expected Error when testing with Regtest
        let values = vec![
            ("6", "\"CONSERVATIVE\""),
            ("18", "\"ECONOMICAL\""),
            ("144", "\"ECONOMICAL\""),
        ];
        let reqs = values.into_iter().map(move |value| {
            let value = value.clone();
            let async_this = this.clone();
            let async_client = rpc_client.clone();
            async move {
                let (a, b) = value.clone();
                let p = vec![a, b];
                let v = async_client
                    .make_rpc_call("estimatesmartfee", &p, false).await;
                let v = v.unwrap();
                match value {
                    ("6", _) => {
                        if let Some(serde_json::Value::Number(hp_btc_per_kb)) = v.get("feerate") {
                            async_this.high_prio_est.store(
                                (hp_btc_per_kb.as_f64().unwrap() * 100_000_000.0 / 250.0) as usize
                                    + 3,
                                Ordering::Release,
                            );
                        };
                    },
                    ("18", _) => {
                        if let Some(serde_json::Value::Number(np_btc_per_kb)) = v.get("feerate") {
                            async_this.normal_est.store(
                                (np_btc_per_kb.as_f64().unwrap() * 100_000_000.0 / 250.0) as usize
                                    + 3,
                                Ordering::Release,
                            );
                        }
                    },
                    ("144", _) => {
                        if let Some(serde_json::Value::Number(bp_btc_per_kb)) = v.get("feerate") {
                            async_this.background_est.store(
                                (bp_btc_per_kb.as_f64().unwrap() * 100_000_000.0 / 250.0) as usize
                                    + 3,
                                Ordering::Release,
                            );
                        }
                    },
                    _ => {},
                }
            }
        });

        future::join_all(reqs).await;
        Ok(())
    }
}
impl chaininterface::FeeEstimator for FeeEstimator {
    fn get_est_sat_per_1000_weight(&self, conf_target: chaininterface::ConfirmationTarget) -> u64 {
        cmp::max(
            match conf_target {
                chaininterface::ConfirmationTarget::Background => {
                    self.background_est.load(Ordering::Acquire) as u64
                }
                chaininterface::ConfirmationTarget::Normal => {
                    self.normal_est.load(Ordering::Acquire) as u64
                }
                chaininterface::ConfirmationTarget::HighPriority => {
                    self.high_prio_est.load(Ordering::Acquire) as u64
                }
            },
            253,
        )
    }
}

pub struct ChainBroadcaster<T> {
    txn_to_broadcast: Mutex<HashMap<Sha256dHash, bitcoin::blockdata::transaction::Transaction>>,
    rpc_client: Arc<RPCClient>,
    larva: T,
}

impl<T> ChainBroadcaster<T> {
    pub fn new(rpc_client: Arc<RPCClient>, larva: T) -> Self {
        Self {
            txn_to_broadcast: Mutex::new(HashMap::new()),
            rpc_client,
            larva,
        }
    }

    fn rebroadcast_txn(&self) -> impl Future {
        let mut send_futures = Vec::new();
        let txn = self.txn_to_broadcast.lock().unwrap();

        for (_, tx) in txn.iter() {
            let tx_ser = "\"".to_string() + &encode::serialize_hex(&tx.clone()) + "\"";
            send_futures.push(async move {
                let tx_ser = [&tx_ser[..]];
                self.rpc_client
                    .make_rpc_call("sendrawtransaction", &tx_ser, true)
                    .map_ok(|_| -> Result<(), ()> { Ok(()) }).await
            });
        }
        block_on(future::join_all(send_futures));
        future::ready(())
    }
}

impl<T: Sync + Send + Larva> chaininterface::BroadcasterInterface for ChainBroadcaster<T> {
    fn broadcast_transaction(&self, tx: &bitcoin::blockdata::transaction::Transaction) {
        self.txn_to_broadcast
            .lock()
            .unwrap()
            .insert(tx.txid(), tx.clone());
        let tx_ser = format!("\"{}\"", &encode::serialize_hex(tx));
        let async_client = self.rpc_client.clone();
        let _ = self.larva.clone().spawn_task(async move {
            let k = &[&tx_ser[..]];
            async_client.make_rpc_call(
                "sendrawtransaction", k, true
            ).map(|_| Ok(())).await
        });
    }
}

enum ForkStep {
    DisconnectBlock(bitcoin::blockdata::block::BlockHeader),
    ConnectBlock((String, u32)),
}

fn find_fork_step(
    mut steps_tx: mpsc::Sender<ForkStep>,
    current_header: GetHeaderResponse,
    target_header_opt: Option<(String, GetHeaderResponse)>,
    rpc_client: Arc<RPCClient>
) {
    debug!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>> FIND FORK STEP >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");
    debug!("current height: {}", &current_header.height);
    debug!("target height: {}", target_header_opt.as_ref().unwrap().1.height);
    debug!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>> FIND FORK STEP >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");

    if target_header_opt.is_some()
        && target_header_opt.as_ref().unwrap().0 == current_header.previousblockhash
    {
        // Target is the parent of current, we're done!
        return
    }
    if current_header.height == 1 {
        return;
    }

    if target_header_opt.is_none()
        || target_header_opt.as_ref().unwrap().1.height < current_header.height
    {
        // currentheader--
        debug!("before try_send");
        let send_res = steps_tx.try_send(ForkStep::ConnectBlock((
            current_header.previousblockhash.clone(),
            current_header.height - 1,
        )));
        match send_res {
            Ok(_) => {
                debug!("try send ok");
                // Spawn a future onto the runtime
                thread::spawn(move || {
                    let mut rt = Runtime::new().unwrap();
                    rt.block_on(async move { 
                        debug!("recursive");
                        let new_cur_header = rpc_client.get_block_header(&current_header.previousblockhash).await;
                        find_fork_step(
                            steps_tx,
                            new_cur_header.unwrap(),
                            target_header_opt,
                            rpc_client
                        );
                    });
                });
                return;

            }
            Err(e) => {
                debug!("try send failed, {}", e);
                return;
            }
        }
    } else {
        // is_some == True 1 2 3 4
        let target_header = target_header_opt.unwrap().1;
        // Everything below needs to disconnect target, so go ahead and do that now
        let c_header = target_header.clone();
        debug!("before try_send 1234" );
        let send_res = block_on(
            steps_tx
                .send(ForkStep::DisconnectBlock(c_header.into()))
        );
        if let Ok(_) = send_res {
            // send err match
            if target_header.previousblockhash == current_header.previousblockhash {
                // Found the fork, also connect current and finish!
                let _ = steps_tx.try_send(ForkStep::ConnectBlock((
                    current_header.previousblockhash.clone(),
                    current_header.height - 1,
                )));
                return;
            } else if target_header.height > current_header.height {
                // Target is higher, walk it back and recurse
                let new_target_header = rpc_client.get_header(&target_header.previousblockhash);
                find_fork_step(
                    steps_tx,
                    current_header,
                    Some((
                        target_header.previousblockhash,
                        new_target_header.unwrap(),
                    )),
                    rpc_client
                );
                return;
            } else {
                // Target and current are at the same height, but we're not at fork yet, walk
                // both back and recurse
                let send_res = steps_tx.try_send(ForkStep::ConnectBlock((
                    current_header.previousblockhash.clone(),
                    current_header.height - 1,
                )));
                if let Ok(_) = send_res {
                    let new_cur_header = rpc_client.get_header(&current_header.previousblockhash);
                    let new_target_header = rpc_client.get_header(&target_header.previousblockhash);
                    find_fork_step(
                        steps_tx,
                        new_cur_header.unwrap(),
                        Some((
                            target_header
                                .previousblockhash,
                            new_target_header
                                .unwrap(),
                        )),
                        rpc_client,
                    );
                    return;
                } else {
                    // Caller droped the receiver, we should give up now
                    return;
                }
            }
        } else {
            // Caller droped the receiver, we should give up now
            return;
        }
    }
}
/// Walks backwards from current_hash and target_hash finding the fork and sending ForkStep events
/// into the steps_tx Sender. There is no ordering guarantee between different ForkStep types, but
/// DisconnectBlock and ConnectBlock events are each in reverse, height-descending order.

async fn find_fork(
    mut steps_tx: mpsc::Sender<ForkStep>,
    current_hash: String,
    target_hash: String,
    rpc_client: Arc<RPCClient>,
) {
    if current_hash == target_hash {
        return;
    }
    let current_resp = rpc_client.get_block_header(&current_hash).await;
    let current_header = current_resp.unwrap();
    if let Ok(_) = steps_tx.start_send(ForkStep::ConnectBlock((
        current_hash.clone(),
        current_header.height
    ))) {
        if current_header.previousblockhash == target_hash || current_header.height == 1 {
            // Fastpath one-new-block-connected or reached block 1
            info!("New block discovered.."); 
            info!("New Height: {}", &current_header.height); 
            info!("New Hash: {}", &current_hash); 
            return;
        } else {
            if let Ok(target_header) = rpc_client.get_block_header(&target_hash).await {
                // Skipped few blocks rescan
                info!("Found non consecutive blocks ... scanning ...");
                find_fork_step(
                    steps_tx,
                    current_header,
                    Some((target_hash, target_header)),
                    rpc_client,
                )
            } else {
                warn!("Unknown block hash Found ...");
                // fork
                assert_eq!(target_hash, "");
                find_fork_step(
                    steps_tx,
                    current_header,
                    None,
                    rpc_client,
                )
            }
        }
    }
}

pub async fn spawn_chain_monitor(
    fee_estimator: Arc<FeeEstimator>,
    rpc_client: Arc<RPCClient>,
    chain_watcher: Arc<ChainWatchInterfaceUtil>,
    chain_broadcaster: Arc<ChainBroadcaster<impl Larva>>,
    event_notify: mpsc::Sender<()>,
    larva: impl Larva,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let _ = larva.clone().spawn_task(async { 
        FeeEstimator::update_values(
            fee_estimator.clone(),
            rpc_client.clone()
        )}.await
    );
    let cur_block = Arc::new(Mutex::new(String::from("")));
    Interval::new(Duration::from_secs(1))
        .for_each(|_| { async {
            let cur_block = cur_block.clone();
            let fee_estimator = fee_estimator.clone();
            let rpc_client = rpc_client.clone();
            let chain_watcher = chain_watcher.clone();
            let chain_broadcaster = chain_broadcaster.clone();
            let mut event_notify = event_notify.clone();
            let larva = larva.clone();
            let _ = larva.spawn_task(async move {
                let v = rpc_client.make_rpc_call("getblockchaininfo", &[], false).await?;
                let new_block = v["bestblockhash"].as_str().unwrap().to_string();
                let old_block = cur_block.lock().unwrap().clone();

                if new_block == old_block {
                    return Ok(());
                }

                *cur_block.lock().unwrap() = new_block.clone();
                if old_block == "" {
                    return Ok(()); 
                }

                let (events_tx, events_rx): (mpsc::Sender<ForkStep>, mpsc::Receiver<ForkStep>) = mpsc::channel(1);

                find_fork(
                    events_tx,
                    new_block,
                    old_block,
                    rpc_client.clone(),
                ).await;

                let events: Vec<ForkStep> = events_rx.collect().await;

                let actions = events.into_iter().rev().map(|event| {
                    let client = rpc_client.clone();
                    let watcher = chain_watcher.clone();
                    async move {
                        match event {
                            ForkStep::DisconnectBlock(ref header) => {
                                info!("Disconnecting block {}", header.bitcoin_hash().to_hex());
                                watcher.block_disconnected(header);
                            }
                            ForkStep::ConnectBlock((ref hash, height)) => {
                                let block_height = height;
                                let param = &[&("\"".to_string() + hash + "\""), "0"];
                                let block_hex = client.make_rpc_call("getblock", param, false).await;
                                let block: Block = encode::deserialize(
                                    &hex_to_vec(block_hex.unwrap().as_str().unwrap())
                                    .unwrap()
                                ).unwrap();
                                info!("Connecting block {}, Height: {}", block.bitcoin_hash().to_hex(), &block_height);
                                watcher.block_connected_with_filtering(&block, block_height);
                            }
                        }
                    }
                });
                
                let _ = future::join_all(actions).await;
                let _ = FeeEstimator::update_values(fee_estimator, rpc_client).await;
                let _ = event_notify.try_send(());
                chain_broadcaster.rebroadcast_txn().await;
                Ok(())
            });
        }
    }).await;
    Ok(())
}
