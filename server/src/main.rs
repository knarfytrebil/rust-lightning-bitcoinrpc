extern crate base64;
extern crate bitcoin;
extern crate bitcoin_bech32;
extern crate bitcoin_hashes;
extern crate bytes;
extern crate config;
extern crate exit_future;
extern crate futures;
extern crate hyper;
extern crate lightning;
extern crate lightning_invoice;
extern crate lightning_net_tokio;
extern crate ln_manager;
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

mod ln_bridge;
mod ln_cmd;

use ln_manager::LnManager;
use ln_manager::executor::Larva;
use futures::future;
use futures::future::Future;

use std::env;
use std::mem;

use ln_manager::ln_bridge::settings::Settings;

#[allow(dead_code, unreachable_code)]
fn _check_usize_is_64() {
    // We assume 64-bit usizes here. If your platform has 32-bit usizes, wtf are you doing?
    unsafe {
        mem::transmute::<*const usize, [u8; 8]>(panic!());
    }
}

#[derive(Clone)]
struct Droid {}

impl Droid {
    fn new() -> Self {
        Droid {}
    }
}

impl Larva for Droid {
    fn spawn_task(
        &self,
        task: impl Future<Item = (), Error = ()> + Send + 'static,
    ) -> Result<(), futures::future::ExecuteError<Box<dyn Future<Item = (), Error = ()> + Send>>>
    {
        Ok(())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // FIXME: Hard code setting argument
    let setting_arg = &args[1];
    println!("USE SETTING FILE - {:?}", setting_arg);
    let settings = Settings::new(setting_arg).unwrap();

    let (signal, exit) = exit_future::signal();
    let droid = Droid::new();
    let ln_manager = LnManager::new(settings, droid, exit);

    // command_handler::run_command_board(ln_manager, executor);

    // rt.shutdown_on_idle().wait().unwrap();
}
