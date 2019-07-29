use base64;
use hyper;
use hyper::rt::{Future as FutureH, Stream as StreamH};
use serde_json;

use bitcoin_hashes::hex::FromHex;
use bitcoin_hashes::sha256d::Hash as Sha256dHash;

use bitcoin::blockdata::block::BlockHeader;

use futures::{future, Future, TryFuture, Stream, FutureExt, StreamExt, TryFutureExt, TryStreamExt};
use futures::executor::block_on;
use futures::future::FutureObj;

use log::{info, error};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Deserialize)]
pub struct GetHeaderResponse {
    pub hash: String,
    pub confirmations: u64,
    pub height: u32,
    pub version: u32,
    pub merkleroot: String,
    pub time: u32,
    pub nonce: u32,
    pub bits: String,
    pub previousblockhash: String,
}

impl GetHeaderResponse {
    pub fn to_block_header(&self) -> BlockHeader {
        BlockHeader {
            version: self.version,
            prev_blockhash: Sha256dHash::from_hex(&self.previousblockhash).unwrap(),
            merkle_root: Sha256dHash::from_hex(&self.merkleroot).unwrap(),
            time: self.time,
            bits: self.bits.parse().unwrap(),
            nonce: self.nonce,
        }
    }
}

pub struct RPCClient {
    basic_auth: String,
    uri: String,
    id: AtomicUsize,
    client: hyper::Client<hyper::client::HttpConnector, hyper::Body>,
}

impl RPCClient {
    pub fn new(rpc_url: String) -> Self {
        let path_parts: Vec<&str> = rpc_url.split('@').collect();
        if path_parts.len() != 2 {
            panic!("Bad RPC URL provided");
        }
        let user_auth = path_parts[0];
        let host_port = path_parts[1];
        Self {
            basic_auth: "Basic ".to_string() + &base64::encode(user_auth),
            uri: "http://".to_string() + host_port,
            id: AtomicUsize::new(0),
            client: hyper::Client::new(),
        }
    }

    /// params entries must be pre-quoted if appropriate
    /// may_fail is only used to change logging
    pub async fn make_rpc_call(
        &self,
        method: &str,
        params: &[&str],
        may_fail: bool,
    ) -> impl Future<Output = Result<serde_json::Value, ()>> {
        let mut request = hyper::Request::post(&self.uri);
        let auth: &str = &self.basic_auth;
        request.header("Authorization", auth);
        let mut param_str = String::new();
        for (idx, param) in params.iter().enumerate() {
            param_str += param;
            if idx != params.len() - 1 {
                param_str += ",";
            }
        }
        self.client
            .request(
                request
                    .body(hyper::Body::from(
                        "{\"method\":\"".to_string()
                            + method
                            + "\",\"params\":["
                            + &param_str
                            + "],\"id\":"
                            + &self.id.fetch_add(1, Ordering::AcqRel).to_string()
                            + "}",
                    ))
                    .unwrap(),
            )
            .map(move |res| {
                let res = res.unwrap();
                if res.status() != hyper::StatusCode::OK {
                    if !may_fail {
                        println!("RPC request failed");
                        println!("{:?}", &res.body());
                        // info!("Failed to get RPC server response (probably bad auth)!");
                    }
                    Err(())
                } else {
                    // return Ok(serde_json::Value::Null);
                    Ok(block_on(res.into_body().try_concat().map_ok(|body| {
                        let v: serde_json::Value = match serde_json::from_slice(&body) {
                            Ok(v) => v,
                            Err(_) => {
                                info!("Failed to parse RPC server response!");
                                // FIXME define error return json value
                                return serde_json::Value::Null;
                            }
                        };
                        if !v.is_object() {
                            info!("Failed to parse RPC server response!");
                            return serde_json::Value::Null;
                        }
                        let v_obj = v.as_object().unwrap();
                        if v_obj.get("error") != Some(&serde_json::Value::Null) {
                            info!("Failed to parse RPC server response!");
                            return serde_json::Value::Null;
                        }
                        if let Some(res) = v_obj.get("result") {
                            return (*res).clone();
                        } else {
                            info!("Failed to parse RPC server response!");
                            return serde_json::Value::Null;
                        }
                    })).unwrap())
                }
            })
    }

    pub fn get_header(
        &self,
        header_hash: &str,
    ) -> impl Future<Output = Result<GetHeaderResponse, ()>> {
        let param = "\"".to_string() + header_hash + "\"";
        self.make_rpc_call("getblockheader", &[&param], false)
            .map(|v| {
                let v = v.unwrap();
                if v.is_object() {
                    if let None = v.get("previousblockhash") {
                        // Got a request for genesis block, add a dummy previousblockhash
                        v.as_object_mut().unwrap().insert(
                            "previousblockhash".to_string(),
                            serde_json::Value::String("".to_string()),
                        );
                    }
                }
                let deser_res: Result<GetHeaderResponse, _> = serde_json::from_value(v);
                match deser_res {
                    Ok(resp) => Ok(resp),
                    Err(_) => {
                        error!("Got invalid header message from RPC server!");
                        Err(())
                    }
                }
            })
    }
}
