use std::net::UdpSocket;
use ln_cmd::tasks::{Probe, ProbT, TaskFn, TaskGen, Action};

fn node() -> Result<(), String> {
    let udp_socket = UdpSocket::bind("0.0.0.0:8123").expect("Could not bind socket");
    println!("hello, test");
    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(node)
}
