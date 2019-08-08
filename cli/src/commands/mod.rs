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

pub fn handle(get_target: &str, sock: std::net::UdpSocket) -> protocol::ResponseFuncs {
    if let Ok(protocol) = get_target.parse() {
        req_rep(
            sock.try_clone().expect("Could not clone socket"),
            protocol
        )
    } else {
        protocol::ResponseFuncs::Error("Invalid Internal Value".to_string())
    }
}
