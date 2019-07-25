#[macro_use]
extern crate clap;

use std::net::UdpSocket;
use std::{thread, time};

use clap::App;

use protocol;

fn req_rep(sock: std::net::UdpSocket, req: protocol::RequestFuncs) -> protocol::ResponseFuncs {
    let msg = protocol::Message::Request(req);
    let ser = protocol::serialize_message(msg);

    sock.send(&ser).expect("Failed to write to server");

    let mut buf = [0u8; 1500];
    let (len, _src) = sock
        .recv_from(&mut buf)
        .expect("Could not read into buffer");
    let buf = &mut buf[..len]; // resize buffer

    let resp = protocol::deserialize_message(buf.to_vec());
    if let protocol::Message::Response(resp) = resp {
        return resp;
    }

    return protocol::ResponseFuncs::Error("No valid response".to_string());
}

fn main() {
    let yaml = load_yaml!("conf/en_US.yml");
    let matches = App::from_yaml(yaml).get_matches();
  
    let socket = UdpSocket::bind("127.0.0.1:5000").expect("Could not bind client socket");
    socket
        .connect("127.0.0.1:8123")
        .expect("Could not connect to server");

    let mut i = 0;
    loop {
        let fmt = format!("Hello Iteration {}", i);
        let resp = req_rep(
            socket.try_clone().expect("Could not clone socket"),
            protocol::RequestFuncs::PrintSomething(fmt),
        );

        println!("{:?}", resp);

        let resp = req_rep(
            socket.try_clone().expect("Could not clone socket"),
            protocol::RequestFuncs::GetRandomNumber,
        );
        println!("{:?}", resp);
        
        i += 1;
        thread::sleep(time::Duration::from_millis(500));
    }
}
