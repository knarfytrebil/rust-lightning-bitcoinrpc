pub mod ln_mgr;
pub mod node;
pub mod udp_srv;
use futures::future::Future;
use futures::task::{Context, Poll};
use futures::FutureExt;

use ln_manager::executor::Larva;
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use crate::ln_node::settings::Settings as NodeSettings;

use std::pin::Pin;

pub type TaskFn = dyn Fn(Vec<Arg>, Probe) -> Result<(), String>;
pub type TaskGen = fn() -> Box<TaskFn>;
pub type Executor = tokio::runtime::TaskExecutor;

#[derive(Clone, Debug)]
pub enum Arg {
    MgrConf(MgrSettings),
    NodeConf(NodeSettings),
}

pub struct Action {
    task_gen: TaskGen,
    args: Vec<Arg>,
    exec: Probe,
}

impl Action {
    pub fn new(task_gen: TaskGen, args: Vec<Arg>, exec: Probe) -> Self {
        Action {
            task_gen: task_gen,
            args: args,
            exec: exec,
        }
    }

    pub fn summon(self) -> Result<(), futures::task::SpawnError> {
        self.exec.clone().spawn_task(self)
    }
}

impl Future for Action {
    type Output = Result<(), ()>;

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        let task = (self.task_gen)();
        match task(self.args.clone(), self.exec.clone()) {
            Ok(_) => Poll::Ready(Ok(())),
            Err(_) => Poll::Pending,
        }
    }
}

#[derive(Clone)]
pub struct Probe {
    exec: Executor,
}

impl Probe {
    pub fn new(exec: Executor) -> Self {
        Probe {
            exec: exec,
        }
    }
}

impl Larva for Probe {
    fn spawn_task(
        &self,
        task: impl Future<Output = Result<(), ()>> + Send + 'static,
    ) -> Result<(), futures::task::SpawnError> {
        self.exec.spawn(async { task.await }.map(|_| ()));
        Ok(())
    }

}
