use protocol;
use rand;
use std::net::UdpSocket;
use std::thread;

fn handle_message(
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
                println!("PrintSomething: {}", s);
                protocol::ResponseFuncs::PrintSomething
            }
            protocol::RequestFuncs::GetRandomNumber => {
                protocol::ResponseFuncs::GetRandomNumber(rand::random())
            }
        }
    }

    let resp_msg = protocol::Message::Response(resp);
    let ser = protocol::serialize_message(resp_msg);
    //println!("Handling connection from {}", src);
    sock.send_to(&ser, &src).expect("Failed to send a response");
}

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:8123").expect("Could not bind socket");

    loop {
        let mut buf = [0u8; 1500];
        let sock = socket.try_clone().expect("Failed to clone socket");
        match socket.recv_from(&mut buf) {
            Ok((sz, src)) => {
                thread::spawn(move || {
                    handle_message(sock, sz, src, buf);
                });
            }
            Err(e) => {
                eprintln!("couldn't recieve a datagram: {}", e);
            }
        }
    }
}
