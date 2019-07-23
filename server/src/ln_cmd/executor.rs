use futures::future::Future;

pub trait Larva: Clone + Sized + Send + Sync + 'static {
    fn spawn_task(
        &self,
        mut task: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), futures::task::SpawnError>;
}
