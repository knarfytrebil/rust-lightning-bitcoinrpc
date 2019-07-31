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

use hyper::Client;
use hyper::Uri;

#[macro_use] 
extern crate failure;
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use serde::{Serialize, Deserialize};

pub type UnboundedSender = mpsc::UnboundedSender<Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>>;

#[derive(Deserialize, Debug)]
struct User {
    id: i32,
    name: String,
}

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

// #[runtime::main]
#[runtime::main(runtime_tokio::Tokio)]
async fn main() {
    let rpc_client = Arc::new(RPCClient::new(String::from("admin2:123@127.0.0.1:19011")));
    let h_client = Arc::new(Client::new());

    let r = runtime::spawn(async move {
        // Interval::new(Duration::from_secs(1))
        //     .for_each(|()|{
        //         // rpc_client.clone().make_rpc_call("getblockchaininfo", &[], false);
        //         future::ready(println!("run task"))
        //     }).await;
        
        // let r = rpc_client.make_rpc_call("getblockchaininfo", &[], false).await;
        // println!("{}", &v.unwrap()); 
        
        let url: Uri = "http://jsonplaceholder.typicode.com/users".parse().unwrap();
        let res = h_client.get(url).await?;
        // asynchronously concatenate chunks of the body
        let body = res.into_body().try_concat().await?;
        // try to parse as json with serde_json
        let users: Vec<User> = serde_json::from_slice(&body)?;

        // println!("======");
        // println!("{:#?}", users);
        
        Ok::<Vec<User>, failure::Error>(users)
    }).await;
    
    println!("{:#?}", r);
}
