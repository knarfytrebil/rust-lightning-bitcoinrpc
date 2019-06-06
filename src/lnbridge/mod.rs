use std::sync::Arc;

use secp256k1::key::{SecretKey};
use bitcoin::network::constants::Network;

use lightning::chain::keysinterface::{KeysManager};
use lightning::chain::chaininterface::{FeeEstimator, ChainWatchInterfaceUtil};
use lightning::ln::peer_handler::{PeerManager};
use lightning::util::logger::{Logger, Level};

pub mod key;

pub mod net_manager;
use self::net_manager::LnSocketDescriptor;

// pub mod event_handler;
pub mod utils;
pub mod channel_monitor;
pub mod channel_manager;
pub mod log_printer;
pub mod rpc_client;
mod fee_estimator;
mod broadcaster;

pub fn arc_keys_manager(&key: &[u8; 32], network: Network, logger: Arc<Logger>) -> Arc<KeysManager> {
    Arc::new(KeysManager::new(&key, network, logger))
}

// pub fn arc_logger(level: Level) -> Arc<Logger> {
//     Arc::new(log_printer::LogPrinter { level })
// }

pub fn arc_fee_estimator() -> Arc<FeeEstimator> {
    Arc::new(fee_estimator::FeeEstimator::new())
}

pub fn arc_chain_watcher(network: Network, logger: Arc<Logger>) -> Arc<ChainWatchInterfaceUtil> {
    Arc::new(ChainWatchInterfaceUtil::new(network, logger))
}

pub fn arc_chain_broadcaster() -> Arc<broadcaster::Broadcaster>{
    Arc::new(broadcaster::Broadcaster::new())
}

// pub fn arc_peer_manager(message_handler, keys: Arc<KeysManager>, logger: Arc<Logger>) -> Arc<PeerManager<LnSocketDescriptor>> {
//     Arc::new(PeerManager::new())
// }
