use std::sync::Arc;
use std::fs;
use std::collections::HashMap;

use secp256k1;

use bitcoin::network::constants::Network;
use bitcoin_hashes::sha256d::Hash;

use lightning::chain::keysinterface::{KeysInterface, KeysManager};
use lightning::chain::chaininterface::{FeeEstimator, ChainWatchInterface, BroadcasterInterface, ChainListener};
use lightning::chain::transaction::OutPoint;
use lightning::ln::channelmanager::{ChannelManager, ChannelManagerReadArgs};
use lightning::ln::channelmonitor::{ChannelMonitor, ManyChannelMonitor};
use lightning::util::ser::ReadableArgs;
use lightning::util::config::UserConfig;
use lightning::util::logger::{Logger};


pub fn get_channel_manager(
    data_path: String,
    network: Network,
    monitors_loaded: Vec<(OutPoint, ChannelMonitor)>,
    keys_manager: Arc<KeysInterface>,
    fee_estimator: Arc<FeeEstimator>,
    monitor: Arc<ManyChannelMonitor>,
    chain_watcher: Arc<ChainWatchInterface>,
    tx_broadcaster: Arc<BroadcasterInterface>,
    logger: Arc<Logger>,
    default_config: UserConfig,
) -> Arc<ChannelManager> {
    if let Ok(mut f) = fs::File::open(data_path + "/manager_data") {
        let (last_block_hash, manager) = {
            let mut monitors_refs = HashMap::new();
            for (outpoint, monitor) in monitors_loaded.iter() {
                monitors_refs.insert(*outpoint, monitor);
            }
            <(Hash, ChannelManager)>::read(&mut f, ChannelManagerReadArgs {
                keys_manager,
                fee_estimator,
                monitor: monitor.clone(),
                chain_monitor: chain_watcher.clone(),
                tx_broadcaster,
                logger, default_config,
                channel_monitors: &monitors_refs,
            }).expect("Failed to deserialize channel manager")
        };

        // monitor.load_from_vec(monitors_loaded);
        let mut mut_monitors_loaded = monitors_loaded;
        for (outpoint, drain_monitor) in mut_monitors_loaded.drain(..) {
            if let Err(_) = monitor.add_update_monitor(outpoint, drain_monitor) {
                panic!("Failed to load monitor that deserialized");
            }
        }
        //TODO: Rescan
        let manager = Arc::new(manager);
        let manager_as_listener: Arc<ChainListener> = manager.clone();
        chain_watcher.register_listener(Arc::downgrade(&manager_as_listener));
        manager
    } else {
        if(!monitors_loaded.is_empty()) {
            panic!("Found some channel monitors but no channel state!");
        }
        ChannelManager::new(network, fee_estimator, monitor, chain_watcher, tx_broadcaster, logger, keys_manager, default_config).unwrap()
    }
}
