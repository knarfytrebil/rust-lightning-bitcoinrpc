use futures::future::Future;
use futures::sync::mpsc;
use futures::Stream;
pub use futures::future::Executor;
use std::marker::Sync;
use std::marker::Sized;
use std::clone::Clone;

pub trait Larva: Executor<Box<dyn Future<Item = (), Error = ()> + Send>> + Clone + Sized + Send + Sync + 'static {
  fn spawn_task(&self, task: impl Future<Item = (), Error = ()> + Send + 'static) -> Result<(), futures::future::ExecuteError<Box<dyn Future<Item = (), Error = ()> + Send>>>{
    self.execute(Box::new(task))
  }
}
