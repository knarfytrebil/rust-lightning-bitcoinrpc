use ln_cmd::tasks::{Probe, ProbT, TaskFn, TaskGen, Action, Arg};
use ln_node::settings::Settings as NodeSettings;
use std::net::UdpSocket;

pub fn task(arg: Vec<Arg>) -> Result<(), String> {
    let node_conf: Option<&NodeSettings> = match &arg[0] {
        Arg::NodeConf(conf) => Some(conf),
        _ => None,
    };
    let node_address = node_conf.unwrap().server.address.clone();
    println!("Lightning Server Running on: {}", &node_address);

    let udp_socket =
        UdpSocket::bind(node_address).expect("Could not bind socket");
    

    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(task)
}
