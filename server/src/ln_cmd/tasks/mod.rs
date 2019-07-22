pub mod node;
pub mod udp_srv;
pub mod ln_mgr;

use futures::future::Future;
use futures::{Async, Poll};
use ln_manager::executor::Larva;

use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_node::settings::Settings as NodeSettings;

use std::thread;
use std::net::UdpSocket;

/* Task Execution Example */
//
// use ln_cmd::tasks::{Probe, ProbT, TaskFn, TaskGen, Action};
//
// let async_exec = Probe::new(ProbT::NonBlocking);
//
// fn test_task() -> Result<(), String> {
//     println!("hello, test");
//     let dur = time::Duration::from_millis(100);
//     thread::sleep(dur);
//     Ok(())
// }
//
// fn test_gen() -> Box<TaskFn> {
//     Box::new(test_task)
// }
//
// let test_action: Action = Action::new(test_gen, false);
//
// async_exec.spawn_task(test_action);
//
/* End of Example */

pub type TaskFn = Fn(Vec<Arg>) -> Result<(), String>;
pub type TaskGen = fn() -> Box<TaskFn>;

#[derive(Clone)]
pub enum ProbT {
    Blocking,
    NonBlocking,
}

#[derive(Clone, Debug)]
pub enum Arg {
    MgrConf(MgrSettings),
    NodeConf(NodeSettings),
}

#[derive(Clone)]
pub struct Probe {
    async: ProbT,
}

impl Probe {
    pub fn new(async: ProbT) -> Self {
        Probe { async: async }
    }
}

pub struct Action {
    done: bool,
    started: bool,
    task_gen: TaskGen,
    args: Vec<Arg>
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
        println!("start");
        let task = (self.task_gen)();
        let _ = task(self.args.clone());
    }
}

impl Future for Action {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if !self.started {
            self.start();
            self.started = true;
        }
        match self.done {
            true => Ok(Async::Ready(())),
            false => Ok(Async::NotReady),
        }
    }
}

impl Larva for Probe {
    fn spawn_task(
        &self,
        mut task: impl Future<Item = (), Error = ()> + Send + 'static,
    ) -> Result<(), futures::future::ExecuteError<Box<dyn Future<Item = (), Error = ()> + Send>>>
    {
        match self.async {
            ProbT::NonBlocking => {
                thread::spawn(move || loop {
                    match task.poll().unwrap() {
                        Async::Ready(_) => {
                            break;
                        }
                        Async::NotReady => {}
                    }
                });
            }
            ProbT::Blocking => loop {
                match task.poll().unwrap() {
                    Async::Ready(_) => {
                        break;
                    }
                    Async::NotReady => {}
                }
            },
        }
        Ok(())
    }
}
