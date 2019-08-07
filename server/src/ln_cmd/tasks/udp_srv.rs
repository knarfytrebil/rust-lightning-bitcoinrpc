use protocol;
use crate::ln_cmd::tasks::{Arg, TaskFn, Probe};
use crate::ln_cmd::help;
use crate::ln_node::settings::Settings as NodeSettings;

use std::net::UdpSocket;
use std::thread;

pub fn task(arg: Vec<Arg>, _exec: Probe) -> Result<(), String> {
    let node_conf: Option<&NodeSettings> = match &arg[0] {
        Arg::NodeConf(conf) => Some(conf),
        _ => None,
    };
    let node_address = node_conf.unwrap().server.address.clone();
    info!("Lightning Server Running on: {}", &node_address);

    let udp_socket = UdpSocket::bind(node_address).expect("Could not bind socket");

    loop {
        let mut buf = [0u8; 1500];
        let sock = udp_socket.try_clone().expect("Failed to clone socket");
        match udp_socket.recv_from(&mut buf) {
            Ok((sz, src)) => {
                thread::spawn(move || {
                    handle_msg(sock, sz, src, buf);
                });
            }
            Err(e) => {
                error!("Couldn't receive a datagram: {}", e);
            }
        }
    }

    // exit here
    // FIXME: Unreachable
    // Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(task)
}

fn handle_msg(
    sock: std::net::UdpSocket,
    sz: usize,
    src: std::net::SocketAddr,
    buf: [u8; 1500],
) {
    let mut vec = buf.to_vec();
    vec.resize(sz, 0);
    let msg = protocol::deserialize_message(vec);
    let mut resp = protocol::ResponseFuncs::Error("Unkown request".to_string());

    if let protocol::Message::Request(msg) = msg {
        resp = match msg {
            protocol::RequestFuncs::PrintSomething(s) => {
                info!("PrintSomething: {}", s);
                protocol::ResponseFuncs::PrintSomething
            }
            protocol::RequestFuncs::GetRandomNumber => {
                protocol::ResponseFuncs::GetRandomNumber(rand::random())
            }
            protocol::RequestFuncs::DisplayHelp => {
                protocol::ResponseFuncs::DisplayHelp(help::get())
            }
        }
    }

    let resp_msg = protocol::Message::Response(resp);
    let ser = protocol::serialize_message(resp_msg);
    debug!("Handling connection from {}", src);
    sock.send_to(&ser, &src).expect("Failed to send a response");
}
