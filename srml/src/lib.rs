extern crate futures;
extern crate hyper;
extern crate bytes;
extern crate base64;
extern crate config;
extern crate exit_future;
extern crate ln_primitives;
extern crate sr_primitives;
extern crate substrate_service;
extern crate ln_manager;

pub use ln_manager::LnManager;

use std::mem;
use std::sync::Arc;
use std::marker::PhantomData;

use futures::future;
use futures::future::Future;
use futures::sync::mpsc;
use exit_future::Exit;

mod lnbridge;
use lnbridge::settings::Settings;

use sr_primitives::traits::{self, ProvideRuntimeApi};
pub use ln_primitives::LnApi;
use substrate_service::{SpawnTaskHandle, Executor};

#[derive(Clone)]
struct Drone {
    spawn_task_handle: SpawnTaskHandle,
    exit: Exit
}
impl Drone {
  fn new(spawn_task_handle: SpawnTaskHandle, exit: Exit) -> Self {
    Self { spawn_task_handle, exit }
  }
}
impl Larva for Drone<T> {
  fn spawn_task(
    &self,
    task: impl Future<Item = (), Error = ()> + Send + 'static
  ) -> Result<(), futures::future::ExecuteError<Box<dyn Future<Item = (), Error = ()> + Send>>> {
    self.spawn_task_handle.execute(Box::new(task.select(exit).then(|_| Ok(()))))
  }
}

pub struct LnBridge<C, Block> {
  client: Arc<C>,
  ln_manager: Arc<LnManager>,
  _block: PhantomData<Block>,
}

impl<C, Block> LnBridge<C, Block> where
  Block: traits::Block,
  C: ProvideRuntimeApi,
  C::Api: LnApi<Block>,
{
  pub fn new(
    client: Arc<C>,
    spawn_task_handle: impl Executor<Box<dyn Future<Item = (), Error = ()> + Send>> + Sync + Send + Clone + 'static,
    exit: Exit
  ) -> Self {
    let settings = Settings::new().unwrap();
    // let client = service.client();
    let drone = Drone::new(spawn_task_handle, exit);
    let ln_manager = Arc::new(LnManager::new(settings, drone));
    Self {
      client,
      ln_manager,
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
