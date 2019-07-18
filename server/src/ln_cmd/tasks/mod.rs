use std::thread;
use futures::future::Future;
use futures::{Async, Poll};
use ln_manager::executor::Larva;

pub type TaskFn = Fn() -> Result<(), String>;
pub type TaskGen = fn() -> Box<TaskFn>;

#[derive(Clone)]
pub struct Probe {}

impl Probe {
    pub fn new() -> Self {
        Probe {}
    }
}

pub struct Action {
    done: bool,
    started: bool,
    task_gen: TaskGen,
}

impl Action {
    pub fn new(task_gen: TaskGen, done: bool) -> Self {
        Action { 
            done: done,
            started: false,
            task_gen: task_gen 
        }
    }

    pub fn start(&self) {
        println!("start");
        (self.task_gen)()();
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
        thread::spawn(move || loop {
            match task.poll().unwrap() {
                Async::Ready(_) => {
                    break;
                }
                Async::NotReady => { }
            }
        });
        Ok(())
    }
}
