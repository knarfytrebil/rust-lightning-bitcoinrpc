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
use ln_cmd::tasks::{Probe, TaskFn, TaskGen, Action};
use ln_manager::executor::Larva;

use std::env;
use std::mem;
use std::{thread, time};

use ln_manager::ln_bridge::settings::Settings;

#[allow(dead_code, unreachable_code)]
fn _check_usize_is_64() {
    // We assume 64-bit usizes here. If your platform has 32-bit usizes, wtf are you doing?
    unsafe {
        mem::transmute::<*const usize, [u8; 8]>(panic!());
    }
}

fn test_task() -> Result<(), String> {
    println!("hello, test");
    let dur = time::Duration::from_millis(100);
    thread::sleep(dur);
    Ok(())
}

fn test_gen() -> Box<TaskFn> {
    Box::new(test_task)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // FIXME: Hard code setting argument
    let setting_arg = &args[1];
    println!("USE SETTING FILE - {:?}", setting_arg);

    let settings = Settings::new(setting_arg).unwrap();

    let probe = Probe::new();

    let test_action: Action = Action::new(test_gen, false);


    probe.spawn_task(test_action);

    let dur = time::Duration::from_millis(10000);
    thread::sleep(dur);
    // let ln_manager = LnManager::new(settings, probe.clone(), exit.clone());
    

    // command_handler::run_command_board(ln_manager, executor);

    // rt.shutdown_on_idle().wait().unwrap();
}
