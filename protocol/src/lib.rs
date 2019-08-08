use bincode;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum RequestFuncs {
    DisplayHelp,
    PrintSomething(String),
    GetRandomNumber,
    GetAddresses,
    GetNodeInfo,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ResponseFuncs {
    DisplayHelp(String),
    PrintSomething,
    GetRandomNumber(i32),
    GetAddresses(String),
    GetNodeInfo(String),
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Message {
    Request(RequestFuncs),
    Response(ResponseFuncs),
}

pub fn serialize_message(msg: Message) -> Vec<u8> {
    bincode::serialize(&msg).expect("Could not serialize message")
}

pub fn deserialize_message(v: Vec<u8>) -> Message {
    bincode::deserialize(&v).expect("Could not deserialize message")
}

pub struct ProtocalParseError {
    pub msg: String
}

impl FromStr for RequestFuncs {
    type Err = ProtocalParseError; 
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "imported_addresses" => {
                Ok(RequestFuncs::GetAddresses)
            }
            "node_info" => {
                Ok(RequestFuncs::GetNodeInfo)
            }
            _ => {
                Err(ProtocalParseError{ msg: String::from("Invalid Request Function") })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let a = Message::Request(RequestFuncs::PrintSomething("".to_string()));
        let ser = serialize_message(a.clone());

        let der = deserialize_message(ser);
        assert_eq!(a, der);
    }
}
