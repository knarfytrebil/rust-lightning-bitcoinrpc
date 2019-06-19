extern crate futures;
extern crate hyper;
extern crate serde_json;
extern crate lightning;
extern crate lightning_net_tokio;
extern crate lightning_invoice;
extern crate rand;
extern crate secp256k1;
extern crate bitcoin;
extern crate tokio;
extern crate tokio_io;
extern crate tokio_fs;
extern crate tokio_codec;
extern crate bytes;
extern crate base64;
extern crate bitcoin_bech32;
extern crate bitcoin_hashes;
extern crate num_traits;
extern crate config;
extern crate exit_future;
extern crate log;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate num_derive;

mod rpc_client;
mod chain_monitor;
mod event_handler;
mod channel_monitor;
mod command_handler;
pub mod ln_manager;
pub use ln_manager::LnManager;

use std::mem;
use std::sync::Arc;

use futures::future;
use futures::future::Future;
use tokio::runtime::TaskExecutor;
use exit_future::Exit;

mod lnbridge;
use lnbridge::settings::Settings;

#[allow(dead_code, unreachable_code)]
fn _check_usize_is_64() {
	// We assume 64-bit usizes here. If your platform has 32-bit usizes, wtf are you doing?
	unsafe { mem::transmute::<*const usize, [u8; 8]>(panic!()); }
}

pub fn run_peer(executor: TaskExecutor, exit: Exit) -> Arc<LnManager> {
  let settings = Settings::new().unwrap();
  let ln_manager = LnManager::new(settings, executor.clone(), exit.clone());
  Arc::new(ln_manager)
}
