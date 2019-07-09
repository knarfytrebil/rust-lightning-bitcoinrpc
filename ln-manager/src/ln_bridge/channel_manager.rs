use std::sync::Arc;
use std::fs;
use std::collections::HashMap;

use bitcoin::network::constants::Network;
use bitcoin_hashes::sha256d::Hash;

use lightning::chain::keysinterface::{KeysInterface};
use lightning::chain::chaininterface::{FeeEstimator, ChainWatchInterface, BroadcasterInterface, ChainListener};
use lightning::chain::transaction::OutPoint;
use lightning::ln::channelmanager::{ChannelManager, ChannelManagerReadArgs};
use lightning::ln::channelmonitor::{ChannelMonitor, ManyChannelMonitor};
use lightning::util::ser::ReadableArgs;
use lightning::util::config::UserConfig;
use lightning::util::logger::{Logger};

use super::Restorable;

const FEE_PROPORTIONAL_MILLIONTHS: u32 = 10;
const ANNOUNCE_CHANNELS: bool = true;

pub struct RestoreArgs {
  data_path: String,
  monitors_loaded: Vec<(OutPoint, ChannelMonitor)>,
  network: Network,
  fee_estimator: Arc<FeeEstimator>,
  monitor: Arc<ManyChannelMonitor>,
  chain_watcher: Arc<ChainWatchInterface>,
  tx_broadcaster: Arc<BroadcasterInterface>,
  logger: Arc<Logger>,
  keys_manager: Arc<KeysInterface>,
}

impl RestoreArgs {
  pub fn new(
    data_path: String,
    monitors_loaded: Vec<(OutPoint, ChannelMonitor)>,
    network: Network,
    fee_estimator: Arc<FeeEstimator>,
    monitor: Arc<ManyChannelMonitor>,
    chain_watcher: Arc<ChainWatchInterface>,
    tx_broadcaster: Arc<BroadcasterInterface>,
    logger: Arc<Logger>,
    keys_manager: Arc<KeysInterface>,
  ) -> Self {
    RestoreArgs {
      data_path, monitors_loaded, network, fee_estimator,
      monitor, chain_watcher, tx_broadcaster,
      logger, keys_manager,
    }
  }
}

impl Restorable<RestoreArgs, Arc<ChannelManager>> for ChannelManager {
  fn try_restore(args: RestoreArgs) -> Arc<ChannelManager> {
    let mut config = UserConfig::new();
		config.channel_options.fee_proportional_millionths = FEE_PROPORTIONAL_MILLIONTHS;
		config.channel_options.announced_channel = ANNOUNCE_CHANNELS;

    if let Ok(mut f) = fs::File::open(args.data_path + "/manager_data") {
      let (last_block_hash, manager) = {
        let mut monitors_refs = HashMap::new();
        for (outpoint, monitor) in args.monitors_loaded.iter() {
          monitors_refs.insert(*outpoint, monitor);
        }
        <(Hash, ChannelManager)>::read(&mut f, ChannelManagerReadArgs {
          keys_manager: args.keys_manager,
          fee_estimator: args.fee_estimator,
          monitor: args.monitor.clone(),
          chain_monitor: args.chain_watcher.clone(),
          tx_broadcaster: args.tx_broadcaster,
          logger: args.logger,
          default_config: config,
          channel_monitors: &monitors_refs,
        }).expect("Failed to deserialize channel manager")
      };

      // monitor.load_from_vec(monitors_loaded);
      let mut mut_monitors_loaded = args.monitors_loaded;
      for (outpoint, drain_monitor) in mut_monitors_loaded.drain(..) {
        if let Err(_) = args.monitor.add_update_monitor(outpoint, drain_monitor) {
          panic!("Failed to load monitor that deserialized");
        }
      }
      //TODO: Rescan
      let manager = Arc::new(manager);
      let manager_as_listener: Arc<ChainListener> = manager.clone();
      args.chain_watcher.register_listener(Arc::downgrade(&manager_as_listener));
      manager
    } else {
      if(!args.monitors_loaded.is_empty()) {
        panic!("Found some channel monitors but no channel state!");
      }
      ChannelManager::new(
        args.network,
        args.fee_estimator,
        args.monitor,
        args.chain_watcher,
        args.tx_broadcaster,
        args.logger, args.keys_manager, config).unwrap()
    }
  }
}
