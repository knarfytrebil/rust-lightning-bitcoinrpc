pub mod ln_mgr;
pub mod node;
pub mod udp_srv;

use futures::channel::mpsc;
use futures::executor::ThreadPool;
use futures::future::Future;
use futures::task::{Context, Poll};
use ln_cmd::executor::SpawnHandler;

use ln_manager::executor::Larva;
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_node::settings::Settings as NodeSettings;

use std::pin::Pin;

pub type TaskFn = Fn(Vec<Arg>, Probe) -> Result<(), String>;
pub type TaskGen = fn() -> Box<TaskFn>;
pub type UnboundedSender = mpsc::UnboundedSender<Pin<Box<dyn Future<Output = ()> + Send>>>;

#[derive(Clone)]
pub enum ProbeT {
    Pool,
}

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
        self.exec.clone().summon_task(self)
    }
}

impl Future for Action {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        let task = (self.task_gen)();
        match task(self.args.clone(), self.exec.clone()) {
            Ok(res) => Poll::Ready(res),
            Err(_) => Poll::Pending,
        }
    }
}

#[derive(Clone)]
pub struct Probe {
    kind: ProbeT,
    sender: UnboundedSender,
    thread_pool: ThreadPool,
}

impl Probe {
    pub fn new(kind: ProbeT, sender: UnboundedSender) -> Self {
        Probe {
            kind: kind,
            sender: sender,
            thread_pool: ThreadPool::new().unwrap(),
        }
    }
}

impl SpawnHandler for Probe {
    fn summon_task(
        &self,
        task: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), futures::task::SpawnError> {
        match self.kind {
            ProbeT::Pool => {
                if let Err(err) = self.sender.unbounded_send(Box::pin(task)) {
                    println!("{}", err);
                    Err(futures::task::SpawnError::shutdown())
                } else {
                    Ok(())
                }
            }
        }
    }
}

// impl Larva for Probe {
//     fn spawn_task(
//         &self,
//         task: impl Future<Item = (), Error = ()> + Send + 'static,
//     ) -> Result<(), futures::future::ExecuteError<Box<dyn Future<Item = (), Error = ()> + Send>>> {
//
//     }
//
// }
