#![feature(async_await)]
use futures::future;
use futures::future::Future;
use futures::prelude::*;
use futures::channel::mpsc;
use futures_timer::Interval;
use futures::executor::{ ThreadPool, LocalPool };
use futures::task::{ LocalSpawn };

use std::time::{Duration};
use std::thread;
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;

use ln_manager::executor::Larva;
use ln_manager::ln_bridge::rpc_client::{ RPCClient };

use hyper::{ Client, Uri };

#[macro_use]
extern crate failure;
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use serde::{Serialize, Deserialize};
pub type UnboundedSender = mpsc::UnboundedSender<Pin<Box<dyn Future<Output = Result<Vec<User>, failure::Error>> + Send>>>;
pub type Executor = tokio::runtime::TaskExecutor;

#[derive(Deserialize, Debug)]
pub struct User {
    id: i32,
    name: String,
}

#[derive(Clone)]
pub struct Probe {
    exec: Executor,
}

// pub trait Larva: Clone + Sized + Send + Sync + 'static {
//     fn spawn_task(
//         &self,
//         // task: impl Future<Output = Result<Vec<User>, failure::Error>> + Send + 'static,
//         task: impl Future<Output = Result<(), ()>> + Send + 'static,
//     ) -> Result<(), futures::task::SpawnError>;
// }

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
        // task: impl Future<Output = Result<Vec<User>, failure::Error>> + Send + 'static,
        task: impl Future<Output = Result<(), ()>> + Send + 'static,
    ) -> Result<(), futures::task::SpawnError> {
        self.exec.spawn(async { task.await }.map(|_| ()));
        Ok(())
    }
}

// impl Larva for Probe {
//     fn spawn_task(
//         &self,
//         task: impl Future<Output = Result<Vec<User>, failure::Error>> + Send + 'static,
//     ) -> Result<(), futures::task::SpawnError> {
//         if let Err(err) = self.sender.start_send(Box::pin(task)) {
//             println!("{}", err);
//             Err(futures::task::SpawnError::shutdown())
//         } else {
//             Ok(())
//         }
//     }
// }


// let rpc_client = Arc::new(RPCClient::new(String::from("admin2:123@127.0.0.1:19011")));
// let r = runtime::spawn(async move {
// }).await;

// Interval::new(Duration::from_secs(1))
//     .for_each(|()|{
//         // rpc_client.clone().make_rpc_call("getblockchaininfo", &[], false);
//         future::ready(println!("run task"))
//     }).await;

// let r = rpc_client.make_rpc_call("getblockchaininfo", &[], false).await;
// println!("{}", &v.unwrap());

async fn h_get_json(i: usize) -> Result<(), ()> {
    // Interval::new(Duration::from_secs(1))
    //     .for_each(|()|{
    //         // rpc_client.clone().make_rpc_call("getblockchaininfo", &[], false);
    //         future::ready(println!("run task"))
    //     }).await;
    // let users = vec![ User { id: 1, name: String::from("Frank") }];
    let h_client = Arc::new(Client::new());
    let url: Uri = "http://jsonplaceholder.typicode.com/users".parse().unwrap();
    let res = h_client.get(url).await.unwrap();
    // // asynchronously concatenate chunks of the body
    let body = res.into_body().try_concat().await.unwrap();
    // // try to parse as json with serde_json
    let users: Vec<User> = serde_json::from_slice(&body).unwrap();

    println!("{}", i);
    // println!("{:#?}", users);
    Ok(())
}

async fn local_rpc() -> Result<(), ()> {
    // let rpc_client = Arc::new(RPCClient::new(String::from("user:pwd@10.146.15.222:18332")));
    let rpc_client = Arc::new(RPCClient::new(String::from("admin1:123@127.0.0.1:19001")));
    let r = rpc_client.make_rpc_call("getblockchaininfo", &[], false).await;
    println!("{:#?}", r);
    // Ok::<Vec<User>, failure::Error>(vec![User{ id: 1, name: String::from("Frank") }])
    Ok(())
}

// #[runtime::main]
// #[runtime::main(runtime_tokio::Tokio)]
// #[tokio::main]
// async fn main() -> Result<(), failure::Error> {
//     let users = h_get_json().await?;
//     println!("{:#?}", users);
//     Ok(())
// }

async fn run_forever() -> Result<(), failure::Error> {
    loop { }
}

fn main() -> Result<(), failure::Error> {
    // runtime::raw::set_runtime(&runtime::native::Native);
    // let (rt_tx, mut rt_rx) = mpsc::unbounded::<Pin<Box<dyn Future<Output = Result<Vec<User>, failure::Error>> + Send>>>();
    // let exec = Probe::new(rt_tx);
    // let _ = exec.clone().spawn_task(async { h_get_json(1).await });
    // let _ = exec.clone().spawn_task(async { h_get_json(2).await });
    // let _ = exec.clone().spawn_task(async { h_get_json(3).await });
    // thread::spawn(move || {
    //     let _ = exec.clone().spawn_task(h_get_json(4));
    // });
    // let _ = exec.clone().spawn_task(async { h_get_json(0).await });
    // let _ = exec.clone().spawn_task(h_get_json(2));
    // let mut pool = LocalPool::new();
    // loop {
    //     match rt_rx.try_next() {
    //         Ok(task) => {
    //             match task {
    //                 Some(t) => {
    //                     tokio_rt.spawn( t.map(|_|{()}) );
    //                 }
    //                 None => {
    //                     println!("we got none");
    //                     break
    //                 }
    //             }
    //             // let r = runtime::spawn(task.unwrap().map(|_|()));
    //             // let r = runtime::raw::enter(runtime_tokio::Tokio, async { task.unwrap().await });
    //             // let r = pool.run_until(async { task.unwrap().await });
    //         }
    //         Err(e) => {
    //             // println!("{:#?}", e);
    //         }
    //     }
    // }

    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let exec = Probe::new(rt.executor());

    //test normal spawn
    exec.clone().spawn_task( async { h_get_json(1).await } );

    // test spawn in thread
    let n = exec.clone();
    thread::spawn(move || {
        n.clone().spawn_task( async { h_get_json(2).await } );
        n.clone().spawn_task( async { local_rpc().await } );
    });

    exec.clone().spawn_task( async { h_get_json(3).await } );
    exec.clone().spawn_task( async { local_rpc().await } );

    rt.block_on(run_forever())
}
