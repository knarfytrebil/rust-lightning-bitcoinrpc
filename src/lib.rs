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
extern crate ln_primitives;

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
use std::marker::PhantomData;

use futures::future;
use futures::future::Future;
use tokio::runtime::TaskExecutor;
use exit_future::Exit;

mod lnbridge;
use lnbridge::settings::Settings;

pub use ln_primitives::LnApi;

// #[allow(dead_code, unreachable_code)]
// fn _check_usize_is_64() {
// 	// We assume 64-bit usizes here. If your platform has 32-bit usizes, wtf are you doing?
// 	unsafe { mem::transmute::<*const usize, [u8; 8]>(panic!()); }
// }

// pub struct LnBridge {}

pub struct LnBridge<C, Block> {
  client: Arc<C>,
  // executor: TaskExecutor,
  // exit: Exit,
  // ln_manager: Arc<LnManager>,
  _block: PhantomData<Block>,
}

impl<C, Block> LnBridge<C, Block> {
  pub fn new(client: Arc<C>) -> Self {
    let settings = Settings::new().unwrap();
    // let ln_manager = Arc::new(LnManager::new(settings, executor.clone(), exit.clone()));
    Self {
      client,
      // executor,
      // exit,
      // ln_manager,
      _block: PhantomData
    }
  }
}

// impl<C, Block> LnBridge<C, Block> where
//   Block: traits::Block,
//   C: ProvideRuntimeApi,
//   C::Api: LnApi<Block>,
// {
//   pub fn on_linked(&self) {
//     // let n = self.client.info().best_number;
//     let runtime_api = self.client.runtime_api();
//   }
// }
