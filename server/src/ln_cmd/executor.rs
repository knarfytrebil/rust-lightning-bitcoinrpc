use futures::future::Future;

pub trait SpawnHandler: Clone + Sized + Send + Sync + 'static {
    fn spawn_task(
        &self,
        task: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), futures::task::SpawnError>;
}
