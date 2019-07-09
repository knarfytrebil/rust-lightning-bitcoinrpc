use futures::future::Future;
use futures::sync::mpsc;
use futures::Stream;
pub use futures::future::Executor;
use std::marker::Sync;
use std::marker::Sized;
use std::clone::Clone;

pub trait TaskExecutor: Executor<Box<dyn Future<Item = (), Error = ()> + Send>> + Clone + Sized + Send + Sync + 'static {}
