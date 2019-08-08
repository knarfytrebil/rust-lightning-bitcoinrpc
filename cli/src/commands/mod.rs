use std::net::UdpSocket;

pub fn req_rep(sock: std::net::UdpSocket, req: protocol::RequestFuncs) -> protocol::ResponseFuncs {
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

fn handle(value: &str, sock: std::net::UdpSocket) -> protocol::ResponseFuncs {
    if let Ok(protocol) = value.parse() {
        req_rep(
            sock.try_clone().expect("Could not clone socket"),
            protocol
        )
    } else {
        protocol::ResponseFuncs::Error("Invalid Internal Value".to_string())
    }
}

pub fn react(command: &str, matches: &clap::ArgMatches ) {
    let socket = 
        UdpSocket::bind("127.0.0.1:5000")
        .expect("Could not bind client socket");

    socket
        .connect("127.0.0.1:8123")
        .expect("Could not connect to server");

    let resp = match matches.value_of(command) {
        Some(value) => {
            let command_and_value = format!("{},{}", command, value);
            handle(&command_and_value, socket)
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
