use futures::future::Future;
use futures::Stream;

pub trait Larva: Clone + Sized + Send + Sync + 'static {
    fn spawn_task(
        &self,
        task: impl Future<Item = (), Error = ()> + Send + 'static,
    ) -> Result<(), futures::future::ExecuteError<Box<dyn Future<Item = (), Error = ()> + Send>>>;
}
