extern crate futures;

use futures::future::Future;
use futures::task::{Context, Poll};
use std::pin::Pin;

pub type TaskFn = Fn(Vec<Arg>) -> Result<(), String>;
pub type TaskGen = fn() -> Box<TaskFn>;

#[derive(Clone)]
pub enum ProbeT {
    Blocking,
    NonBlocking,
    Pool,
}

#[derive(Clone, Debug)]
pub enum Arg {
    Conf(String),
}
struct Task {
    done: bool,
    started: bool,
    task_gen: TaskGen,
    args: Vec<Arg>,
}

impl Task {
    pub fn new(task_gen: TaskGen, args: Vec<Arg>) -> Self {
        Task {
            done: false,
            started: false,
            task_gen: task_gen,
            args: args,
        }
    }

    fn start(&self) {
        let task = (self.task_gen)();
        let _ = task(self.args.clone());
    }
}

impl Future for Task {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
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

fn test_task(args: Vec<Arg>) -> Result<(), String> {
    println!("hi from task");
    Ok(())
}

fn gen() -> Box<TaskFn> {
    Box::new(test_task)
}

fn main() {
    println!("Hello, world!");
    let t = Task::new(gen, vec![Arg::Conf(String::from("frank"))]);
}
