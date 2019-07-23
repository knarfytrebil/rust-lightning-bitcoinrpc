pub mod ln_mgr;
pub mod node;
pub mod udp_srv;

use futures::channel::mpsc;
use futures::future::Future;
use futures::task::{Context, Poll};
use futures::executor::ThreadPool;
use ln_cmd::executor::Larva;

use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_node::settings::Settings as NodeSettings;

use std::pin::Pin;
use std::{panic, thread};

pub type TaskFn = Fn(Vec<Arg>) -> Result<(), String>;
pub type TaskGen = fn() -> Box<TaskFn>;
pub type UnboundedSender = mpsc::UnboundedSender<Box<dyn Future<Output = ()> + Send>>;

#[derive(Clone)]
pub enum ProbeT {
    Blocking,
    NonBlocking,
    Pool,
}

#[derive(Clone, Debug)]
pub enum Arg {
    MgrConf(MgrSettings),
    NodeConf(NodeSettings),
}

#[derive(Clone)]
pub struct Probe {
    kind: ProbeT,
    sender: UnboundedSender,
}

impl Probe {
    pub fn new(kind: ProbeT, sender: UnboundedSender) -> Self {
        Probe {
            kind: kind,
            sender: sender,
        }
    }
}

pub struct Action {
    done: bool,
    started: bool,
    task_gen: TaskGen,
    args: Vec<Arg>,
}

impl Action {
    pub fn new(task_gen: TaskGen, args: Vec<Arg>) -> Self {
        Action {
            done: false,
            started: false,
            task_gen: task_gen,
            args: args,
        }
    }

    pub fn start(&self) {
        let task = (self.task_gen)();
        let _ = task(self.args.clone());
    }
}

impl Future for Action {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.started {
            self.start();
            self.started = true;
        }
        match self.done {
            true => Poll::Ready(()),
            false => Poll::Pending,
        }
    }
}

impl Larva for Probe {
    fn spawn_task(
        &self,
        task: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), futures::task::SpawnError> {

        // panic handler
        panic::set_hook(Box::new(|panic_info| {
            println!("{:?}", &panic_info);
        }));

        match self.kind {
            ProbeT::NonBlocking => {
                thread::spawn(move || loop {
                    match task.poll() {
                        Err(err) => {
                            return err;
                        }
                        Ok(res) => match res {
                            Poll::Ready(r) => {
                                return r;
                            }
                            Poll::Pending => {}
                        },
                    }
                });
            }
            ProbeT::Blocking => loop {
                match task.poll() {
                    Err(err) => {
                        println!("{:?}", err);
                        break;
                    }
                    Ok(res) => match res {
                        Poll::Ready(_) => {
                            break;
                        }
                        Poll::Pending => {}
                    },
                }
            },
            ProbeT::Pool => {}
        }
        Ok(())
    }
}
