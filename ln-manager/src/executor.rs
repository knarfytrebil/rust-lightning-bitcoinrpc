use futures::future::Future;
use futures::sync::mpsc;
use futures::Stream;
pub use futures::future::Executor;
use std::marker::Sync;
use std::clone::Clone;
// use substrate_service::{SpawnTaskHandle, Executor};

pub type FutureExecutor = Executor<Box<dyn Future<Item = (), Error = ()> + Send>>;

#[derive(Clone)]
pub struct TaskExecutor {}

// pub trait TaskExecutor {
//   fn spawn(&self, future: Box<dyn Future<Item = (), Error = ()> + Send>);
// }
