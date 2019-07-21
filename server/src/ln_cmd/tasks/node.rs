use ln_cmd::tasks::{Action, Arg, ProbT, Probe, TaskFn, TaskGen};
use ln_node::settings::Settings as NodeSettings;
use std::net::UdpSocket;

// arg.0 = ln_conf
// arg.1 = node_conf
fn node(arg: Vec<Arg>) -> Result<(), String> {
    let node_conf: Option<&NodeSettings> = match &arg[1] {
        Arg::NodeConf(conf) => Some(conf),
        _ => None,
    };
    let node_address = node_conf.unwrap().server.address.clone();
    println!("{}", &node_address);

    let udp_socket =
        UdpSocket::bind(node_address).expect("Could not bind socket");

    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(node)
}
