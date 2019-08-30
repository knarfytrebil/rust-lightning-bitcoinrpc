extern crate base64;
extern crate bitcoin;
extern crate bitcoin_bech32;
extern crate bitcoin_hashes;
extern crate bytes;
extern crate config;
extern crate failure;
extern crate futures;
extern crate hyper;
extern crate lightning;
extern crate lightning_invoice;
extern crate ln_manager;
extern crate num_traits;
extern crate protocol;
extern crate rand;
extern crate secp256k1;
extern crate serde_json;

#[macro_use]
extern crate log;
extern crate simplelog;

#[macro_use]
extern crate serde_derive;
extern crate num_derive;

mod ln_cmd;
mod ln_node;

use std::env;
use std::fs::File;
use std::mem;

use simplelog::*;

use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_node::settings::Settings as NodeSettings;

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

    let ln_conf = MgrSettings::new(ln_conf_arg).unwrap();
    let node_conf = NodeSettings::new(node_conf_arg).unwrap();

    let log_file_name = format!("server_{}.log", &node_conf.server.address);

    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed).unwrap(),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create(&log_file_name).unwrap(),
        ),
    ]).unwrap();

    info!("reading ln SETTING FILE - {:?}", ln_conf_arg);
    info!("reading node SETTING FILE - {:?}", node_conf_arg);
    info!("log printed to {:?}", log_file_name);
    ln_node::run(ln_conf, node_conf);
}
