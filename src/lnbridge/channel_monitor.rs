use std::fs;
use std::vec::Vec;
use std::sync::{Arc};
use std::io::{Cursor, Write};
use bitcoin::blockdata::transaction::{Transaction};
use bitcoin_hashes::sha256d::Hash;
use bitcoin_hashes::hex::FromHex;
use lightning::chain::chaininterface::BroadcasterInterface;
use lightning::chain::transaction::OutPoint;
use lightning::ln::channelmonitor;
use lightning::ln::channelmonitor::{ChannelMonitor, ChannelMonitorUpdateErr};
use lightning::util::logger::Logger;
use lightning::util::ser::ReadableArgs;

pub struct Broadcaster {
}

impl Broadcaster {
  pub fn new() -> Self {
    Self {}
  }
}

impl BroadcasterInterface for Broadcaster {
  fn broadcast_transaction(&self, tx: &Transaction) {
    // sendrawtx
    println!("broadcast tx");
  }
}

pub fn load_from_disk(data_path: String, logger: Arc<Logger>) -> Vec<(OutPoint, ChannelMonitor)> {
  let file_prefix = data_path + "/monitors";
  let mut res = Vec::new();
  for file_option in fs::read_dir(file_prefix).unwrap() {
    let mut loaded = false;
    let file = file_option.unwrap();
    if let Some(filename) = file.file_name().to_str() {
      if filename.is_ascii() && filename.len() > 65 {
        if let (
          Ok(txid),
          Ok(index),
          Ok(contents)
        ) = (
          Hash::from_hex(filename.split_at(64).0),
          filename.split_at(65).1.split('.').next().unwrap().parse(),
          fs::read(&file.path())
        ) {
              if let Ok((last_block_hash, loaded_monitor)) = <(Hash, ChannelMonitor)>::read(&mut Cursor::new(&contents), logger.clone()) {
                res.push((OutPoint { txid, index }, loaded_monitor));
                loaded = true
              }}}}
    if !loaded {
      println!("WARNING: Failed to read one of the channel monitor storage files! Check perms");
    }
  }
  res
}

