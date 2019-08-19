pub fn json(resp: protocol::ResponseFuncs) {
    use serde_json::json;
    let res = match resp {
        protocol::ResponseFuncs::GetAddresses(addrs) => {
            json!({ "imported_addresses": addrs })
        }
        protocol::ResponseFuncs::GetNodeInfo(info) => {
            json!({ "node_id": info })
        }
        protocol::ResponseFuncs::PeerConnect => {
            json!({ "response": "Request Acknowledged ..."})
        }
        protocol::ResponseFuncs::PeerList(peers) => {
            json!({ "peers": peers })
        }
        protocol::ResponseFuncs::InvoiceCreate(res) => {
            match res {
                Ok(invoice) => {
                    json!({ 
                        "response": "Invoice Created ...",
                        "invoice": invoice
                    })
                }
                Err(e) => {
                    json!({ 
                        "response": "Invoice Creation Error ...",
                        "error": e 
                    })
                }
            }
        }
        protocol::ResponseFuncs::Error(e) => {
            json!({ 
                "response": "Error",
                "error": e 
            })
        }
        protocol::ResponseFuncs::ChannelCreate(c) => {
            json!({ "channel": c })
        }
        _ => {
            json!({ 
                "response": "Error",
                "error": "Unknwon Protocol" 
            })
        }
    };
    println!("{}", serde_json::to_string_pretty(&res).unwrap());
}

pub fn human(resp: protocol::ResponseFuncs) {
    match resp {
        protocol::ResponseFuncs::GetAddresses(addrs) => {
            println!("Imported Addresses:"); 
            for addr in addrs {
                println!("{}", addr);
            }
        }
        protocol::ResponseFuncs::GetNodeInfo(info) => {
            println!("{}", info);
        }
        protocol::ResponseFuncs::PeerConnect => {
            println!("Request Acknowledged ...");
        }
        protocol::ResponseFuncs::PeerList(peers) => {
            println!("Connected Peers:");
            for peer in peers {
                println!("{}", peer);
            }
        }
        protocol::ResponseFuncs::InvoiceCreate(res) => {
            match res {
                Ok(invoice) => {
                    println!("Invoice created");
                    println!("{}", invoice);
                }
                Err(e) => {
                    println!("Invoice creation error");
                    println!("{}", e);
                }
            }
        }
        protocol::ResponseFuncs::Error(e) => {
            println!("{}", e);
        }
        _ => {}
    };
}
