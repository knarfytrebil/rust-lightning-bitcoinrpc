use std::net::UdpSocket;
mod output;

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

pub fn react(command: &str, sub_command: &str, matches: &clap::ArgMatches, sub_matches: &clap::ArgMatches) {
    let node_addr = matches
        .value_of("node")
        .unwrap_or("127.0.0.1:8123");

    let fn_output_format = match matches.is_present("json") {
        true => output::json,
        false => output::human
    };

    let socket =
        UdpSocket::bind("0.0.0.0:5000")
        .expect("Could not bind client socket");

    socket
        .connect(node_addr)
        .expect("Could not connect to server");

    // println!("matches:{:#?}", &matches);
    // println!("sub_matches:{:#?}", &sub_matches);
    // println!("command:{}", &command);
    // println!("sub_command:{}", &sub_command);

    let resp = match sub_matches.values_of(sub_command) {
        Some(values) => {
            let value: Vec<String> = values
                .into_iter()
                .map(|v| {
                    v.to_string()
                })
                .collect();
            let command_and_value = format!("{},{},{}", command, sub_command, value.join(","));
            handle(&command_and_value, socket)
        }
        _ => {
            protocol::ResponseFuncs::Error("Invalid Command or Arguments Provided\nTry running with --help or -h".to_string())
        }
    };

    fn_output_format(resp);
}
