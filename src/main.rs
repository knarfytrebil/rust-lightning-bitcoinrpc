extern crate base64;
extern crate bitcoin;
extern crate bitcoin_bech32;
extern crate bitcoin_hashes;
extern crate bytes;
extern crate config;
extern crate futures;
extern crate hyper;
extern crate lightning;
extern crate lightning_invoice;
extern crate lightning_net_tokio;
extern crate log;
extern crate num_traits;
extern crate rand;
extern crate secp256k1;
extern crate serde_json;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_fs;
extern crate tokio_io;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate num_derive;

mod chain_monitor;
mod channel_monitor;
mod command_handler;
mod event_handler;
mod ln_manager;
mod rpc_client;
use ln_manager::LnManager;

use std::mem;
use std::env;

use futures::future;
use futures::future::Future;
use tokio::runtime::TaskExecutor;

mod lnbridge;
use lnbridge::settings::Settings;

#[allow(dead_code, unreachable_code)]
fn _check_usize_is_64() {
    // We assume 64-bit usizes here. If your platform has 32-bit usizes, wtf are you doing?
    unsafe {
        mem::transmute::<*const usize, [u8; 8]>(panic!());
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // FIXME: Hard code setting argument
    let setting_arg = &args[1];
    println!("USE SETTING FILE - {:?}", setting_arg);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let executor = rt.executor();
    let settings = Settings::new(setting_arg).unwrap();
    let lnManager = LnManager::new(settings, executor.clone());

    command_handler::run_command_board(lnManager, executor);

    rt.shutdown_on_idle().wait().unwrap();
}