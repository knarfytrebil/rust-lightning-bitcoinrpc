use bincode;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum RequestFuncs {
    DisplayHelp,
    GetAddresses,
    GetNodeInfo,
    PeerConnect(String),
    ChannelCreate(Vec<String>),
    ChannelClose(String),
    ChannelCloseAll,
    ChannelList,
    InvoiceCreate(String),
    PeerList,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ResponseFuncs {
    DisplayHelp(String),
    GetAddresses(Vec<String>),
    GetNodeInfo(String),
    PeerConnect,
    ChannelCreate,
    ChannelClose,
    ChannelCloseAll,
    ChannelList,
    PeerList(Vec<String>),
    InvoiceCreate(String),
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
        let cmd_value: Vec<&str> = s.split(',').collect();
        let cmd = cmd_value[0];
        let sub_command = cmd_value[1];
        match cmd {
            "info" => {
                 match sub_command {
                    "addresses" => {
                        Ok(RequestFuncs::GetAddresses)
                    }
                    "node" => {
                        Ok(RequestFuncs::GetNodeInfo)
                    }
                    _ => {
                        Err(ProtocalParseError{ msg: String::from("Invalid Argument") })
                    }
                }
            }
            "peer" => {
                match sub_command {
                    "connect" => {
                        let value = cmd_value[2].to_string();
                        Ok(RequestFuncs::PeerConnect(value))
                    }
                    "list" => {
                        Ok(RequestFuncs::PeerList)
                    }
                    _ => {
                        Err(ProtocalParseError{ msg: String::from("Invalid Argument") })
                    }
                }
            }
            "channel" => {
                match sub_command {
                    "create" => {
                        if cmd_value.len() != 5 {
                            return Err(ProtocalParseError{ msg: String::from("Insufficient Arguments") });
                        }
                        let args: Vec<String> = cmd_value[2..]
                            .into_iter()
                            .map(|v| {
                                v.to_string()
                            }).collect();
                        Ok(RequestFuncs::ChannelCreate(args))
                    }
                    "kill" => {
                        if cmd_value.len() != 3 {
                            return Err(ProtocalParseError{ msg: String::from("Insufficient Arguments") });
                        }
                        let channel = cmd_value[2].to_string();
                        Ok(RequestFuncs::ChannelClose(channel))
                    }
                    "killall" => {
                        Ok(RequestFuncs::ChannelCloseAll)
                    }
                    "list" => {
                        Ok(RequestFuncs::ChannelList)
                    }
                    _ => {
                        Err(ProtocalParseError{ msg: String::from("Invalid Value") })
                    }
                }
            }
            _ => {
                Err(ProtocalParseError{ msg: String::from("Invalid Command") })
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
