pub fn json(resp: protocol::ResponseFuncs) {
    use serde_json::json;
    let res = match resp {
        protocol::ResponseFuncs::GetAddresses(addrs) => {
            json!({ "imported_addresses": addrs })
        }
        protocol::ResponseFuncs::GetNodeInfo(info) => {
            json!({ "node_id": info })
        }
        protocol::ResponseFuncs::PeerConnect | protocol::ResponseFuncs::ChannelCloseAll => {
            json!({ "response": "Request Acknowledged ..."})
        }
        protocol::ResponseFuncs::PeerList(peers) => {
            json!({ "peers": peers })
        }
        protocol::ResponseFuncs::InvoiceCreate(invoice) => {
            json!({ 
                "response": "Invoice Created ...",
                "invoice": invoice
            })
        }
        protocol::ResponseFuncs::ChannelCreate(c) => {
            json!({ "channel": c })
        }
        protocol::ResponseFuncs::ChannelClose(c) => {
            json!({ 
                "response": "Channel closed",
                "channel": c,
            })
        }
        protocol::ResponseFuncs::ChannelList(l) => {
            let channels: Vec<serde_json::Value> = l.into_iter().map(|c|{
                serde_json::from_str(&c).unwrap()
            }).collect();
            json!({ 
                "channels": channels 
            })
        }
        protocol::ResponseFuncs::InvoicePay => {
            json!({ "response": "Invoice Paid" })
        }
        protocol::ResponseFuncs::Error(e) => {
            json!({ 
                "response": "Error",
                "error": e 
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
        protocol::ResponseFuncs::InvoiceCreate(invoice) => {
            println!("Invoice created");
            println!("{}", invoice);
        }
        protocol::ResponseFuncs::Error(e) => {
            println!("{}", e);
        }
        _ => {}
    };
}
