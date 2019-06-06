use base64;
use hyper;

use std::sync::atomic::{AtomicUsize};

pub struct RPCClient {
    basic_auth: String,
    uri: String,
    id: AtomicUsize,
    client: hyper::Client<hyper::client::HttpConnector, hyper::Body>,
}

impl RPCClient {
    pub fn new(user_auth: &str, host_port: &str) -> Self {
        Self {
            basic_auth: "Basic ".to_string() + &base64::encode(user_auth),
            uri: "http://".to_string() + host_port,
            id: AtomicUsize::new(0),
            client: hyper::Client::new(),
        }
    }
}
