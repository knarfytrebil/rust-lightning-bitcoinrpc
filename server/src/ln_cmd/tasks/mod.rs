use futures::future;
use futures::future::Future;
use futures::{Async, Poll};
use ln_manager::executor::Larva;

#[derive(Clone)]
pub struct Probe {}

impl Probe {
    pub fn new() -> Self {
        Probe {}
    }
}

#[derive(Default)]
struct Action {
    done: bool,
}

impl Future for Action {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        println!("Done: {}", self.done);
        match self.done {
            true => Ok(Async::Ready(())),
            false => Ok(Async::NotReady),
        }
    }
}

impl Larva for Probe {
    fn spawn_task(
        &self,
        task: impl Future<Item = (), Error = ()> + Send + 'static,
    ) -> Result<(), futures::future::ExecuteError<Box<dyn Future<Item = (), Error = ()> + Send>>>
    {
        Ok(())
    }
}
