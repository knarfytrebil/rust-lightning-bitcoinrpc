#[macro_use]
extern crate clap;

use std::net::UdpSocket;
use clap::App;

use protocol;
mod commands;

fn main() {
    // Load Command Mappings
    let yaml = load_yaml!("conf/en_US.yml");
    let matches = App::from_yaml(yaml).get_matches();
  
    // Establish Socket Connection with Udp Server
    let socket = 
        UdpSocket::bind("127.0.0.1:5000")
        .expect("Could not bind client socket");

    socket
        .connect("127.0.0.1:8123")
        .expect("Could not connect to server");

    let resp = match matches.value_of("get") {
        Some(get_target) => {
            commands::handle(get_target,socket)
        }
        _ => {
            protocol::ResponseFuncs::Error("Invalid Command or Arguments Provided\nTry running with --help or -h".to_string())
        }
    };

    match resp {
        protocol::ResponseFuncs::GetAddresses(addrs) => {
            println!("{}", addrs);
        }
        protocol::ResponseFuncs::GetNodeInfo(info) => {
            println!("{}", info);
        }
        protocol::ResponseFuncs::Error(e) => {
            println!("{}", e);
        }
        _ => {}
    };

}
