#![feature(async_await)]
use futures::future;
use futures::prelude::*;
use futures::channel::mpsc;
use futures_timer::Interval;
use futures::executor::ThreadPool;
use futures::executor::LocalPool;

use std::time::{Duration};
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;

use ln_manager::executor::Larva;
use ln_manager::ln_bridge::rpc_client::{RPCClient};

pub type UnboundedSender = mpsc::UnboundedSender<Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>>;

#[derive(Clone)]
pub struct Probe {
    sender: UnboundedSender,
    thread_pool: ThreadPool,
}

impl Probe {
    pub fn new(sender: UnboundedSender) -> Self {
        Probe {
            sender: sender,
            thread_pool: ThreadPool::new().unwrap(),
        }
    }
}

impl Larva for Probe {
    fn spawn_task(
        &self,
        task: impl Future<Output = Result<(), ()>> + Send + 'static,
    ) -> Result<(), futures::task::SpawnError> {
        if let Err(err) = self.sender.unbounded_send(Box::pin(task)) {
            println!("{}", err);
            Err(futures::task::SpawnError::shutdown())
        } else {
            Ok(())
        }
    }
}

fn main() {
    let (pool_tx, mut pool_rx) = mpsc::unbounded::<Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>>();
    let runner = Probe::new(pool_tx);
    let rpc_client = Arc::new(RPCClient::new(String::from("admin2:123@127.0.0.1:19011")));

    runner.spawn_task(async move {
        // Interval::new(Duration::from_secs(1))
        // .for_each(|()|{
        //     // rpc_client.clone().make_rpc_call("getblockchaininfo", &[], false);
        //     future::ready(println!("run task"))
        // }).await;
        let v = rpc_client.make_rpc_call("getblockchaininfo", &[], false).await;
        println!("{}", &v.unwrap()); 
        Ok(())
    });
   
    let mut pool = LocalPool::new();

    loop {
        match pool_rx.try_next() {
            Ok(task) => {
                let _ = pool.run_until(task.unwrap());
            }
            _ => {}
        }
    }

}
