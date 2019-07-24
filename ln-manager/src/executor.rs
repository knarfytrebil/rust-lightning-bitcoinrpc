use futures::future::Future;

pub trait Larva: Clone + Sized + Send + Sync + 'static {
    fn spawn_task(
        &self,
        task: impl Future<Output = Result<(), ()>> + Send + 'static,
    ) -> Result<(), futures::task::SpawnError>;
}
