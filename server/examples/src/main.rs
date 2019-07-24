extern crate futures;

use futures::future::Future;
use futures::task::{Context, Poll};
use futures::executor::LocalPool;
use std::pin::Pin;

pub type TaskFn = Fn(Vec<Arg>) -> Result<bool, String>;
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
    task_gen: TaskGen,
    args: Vec<Arg>,
}

impl Task {
    pub fn new(task_gen: TaskGen, args: Vec<Arg>) -> Self {
        Task {
            task_gen: task_gen,
            args: args,
        }
    }
}

impl Future for Task {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        let task = (self.task_gen)();
        match task(self.args.clone()) {
            Ok(res) => Poll::Ready(res),
            Err(error) => Poll::Pending,
        }
    }
}

fn test_task(args: Vec<Arg>) -> Result<bool, String> {
    println!("hi from task");
    for i in args.iter() {
        println!("{:?}", i);
    }
    Ok(true)
}

fn gen() -> Box<TaskFn> {
    Box::new(test_task)
}

fn main() {
    println!("Hello, world!");

    let t = Task::new(gen, vec![Arg::Conf(String::from("frank"))]);
    let mut pool = LocalPool::new();

    pool.run_until(t);
}
