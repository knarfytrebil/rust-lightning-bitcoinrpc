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

mod ln_cmd;
mod ln_node;

use std::env;
use std::mem;

use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_node::settings::Settings as NodeSettings;
// use ln_manager::LnManager;

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
    let ln_conf_arg = &args[1];
    let node_conf_arg = &args[2];

    println!("USE ln SETTING FILE - {:?}", ln_conf_arg);
    println!("USE node SETTING FILE - {:?}", node_conf_arg);

    let ln_conf = MgrSettings::new(ln_conf_arg).unwrap();
    let node_conf = NodeSettings::new(node_conf_arg).unwrap();

    // println!("{:#?}", ln_conf);
    // println!("{:#?}", node_conf);

    ln_node::run(ln_conf, node_conf);

    // let ln_manager = LnManager::new(settings, probe.clone(), exit.clone());

    // command_handler::run_command_board(ln_manager, executor);

    // rt.shutdown_on_idle().wait().unwrap();
}
