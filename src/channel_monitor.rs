use std::fs;
use std::sync::Arc;
use std::io::{Cursor};

use bitcoin_hashes::hex::{ToHex, FromHex};
use bitcoin_hashes::sha256d::Hash as Sha256dHash;

use lightning::chain;
use lightning::ln::channelmonitor;
use lightning::ln::channelmonitor::ManyChannelMonitor;
use lightning::util::ser::ReadableArgs;

use ln_bridge::log_printer::LogPrinter;
use log::{info};

pub struct ChannelMonitor {
	pub monitor: Arc<channelmonitor::SimpleManyChannelMonitor<chain::transaction::OutPoint>>,
	pub file_prefix: String,
}
impl ChannelMonitor {
	pub fn load_from_disk(file_prefix: &String) -> Vec<(chain::transaction::OutPoint, channelmonitor::ChannelMonitor)> {
		let mut res = Vec::new();
		for file_option in fs::read_dir(file_prefix).unwrap() {
			let mut loaded = false;
			let file = file_option.unwrap();
			if let Some(filename) = file.file_name().to_str() {
				if filename.is_ascii() && filename.len() > 65 {
					if let Ok(txid) = Sha256dHash::from_hex(filename.split_at(64).0) {
						if let Ok(index) = filename.split_at(65).1.split('.').next().unwrap().parse() {
							if let Ok(contents) = fs::read(&file.path()) {
								if let Ok((last_block_hash, loaded_monitor)) = <(Sha256dHash, channelmonitor::ChannelMonitor)>::read(&mut Cursor::new(&contents), Arc::new(LogPrinter{})) {
									// TODO: Rescan from last_block_hash
									res.push((chain::transaction::OutPoint { txid, index }, loaded_monitor));
									loaded = true;
								}
							}
						}
					}
				}
			}
			if !loaded {
				info!("WARNING: Failed to read one of the channel monitor storage files! Check perms!");
			}
		}
		res
	}

	pub fn load_from_vec(&self, mut monitors: Vec<(chain::transaction::OutPoint, channelmonitor::ChannelMonitor)>) {
		for (outpoint, monitor) in monitors.drain(..) {
			if let Err(_) = self.monitor.add_update_monitor(outpoint, monitor) {
				panic!("Failed to load monitor that deserialized");
			}
		}
	}
}
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[error("OSX creatively eats your data, using Lightning on OSX is unsafe")]
struct ERR {}

impl channelmonitor::ManyChannelMonitor for ChannelMonitor {
	fn add_update_monitor(&self, funding_txo: chain::transaction::OutPoint, monitor: channelmonitor::ChannelMonitor) -> Result<(), channelmonitor::ChannelMonitorUpdateErr> {
		macro_rules! try_fs {
			($res: expr) => {
				match $res {
					Ok(res) => res,
					Err(_) => return Err(channelmonitor::ChannelMonitorUpdateErr::PermanentFailure),
				}
			}
		}
		// Do a crazy dance with lots of fsync()s to be overly cautious here...
		// We never want to end up in a state where we've lost the old data, or end up using the
		// old data on power loss after we've returned
		// Note that this actually *isn't* enough (at least on Linux)! We need to fsync an fd with
		// the containing dir, but Rust doesn't let us do that directly, sadly. TODO: Fix this with
		// the libc crate!
		let filename = format!("{}/{}_{}", self.file_prefix, funding_txo.txid.to_hex(), funding_txo.index);
		let tmp_filename = filename.clone() + ".tmp";

		{
			let mut f = try_fs!(fs::File::create(&tmp_filename));
			try_fs!(monitor.write_for_disk(&mut f));
			try_fs!(f.sync_all());
		}
		// We don't need to create a backup if didn't already have the file, but in any other case
		// try to create the backup and expect failure on fs::copy() if eg there's a perms issue.
		let need_bk = match fs::metadata(&filename) {
			Ok(data) => {
				if !data.is_file() { return Err(channelmonitor::ChannelMonitorUpdateErr::PermanentFailure); }
				true
			},
			Err(e) => match e.kind() {
				std::io::ErrorKind::NotFound => false,
				_ => true,
			}
		};
		let bk_filename = filename.clone() + ".bk";
		if need_bk {
			try_fs!(fs::copy(&filename, &bk_filename));
			{
				let f = try_fs!(fs::File::open(&bk_filename));
				try_fs!(f.sync_all());
			}
		}
		try_fs!(fs::rename(&tmp_filename, &filename));
		{
			let f = try_fs!(fs::File::open(&filename));
			try_fs!(f.sync_all());
		}
		if need_bk {
			try_fs!(fs::remove_file(&bk_filename));
		}
		self.monitor.add_update_monitor(funding_txo, monitor)
	}

	fn fetch_pending_htlc_updated(&self) -> Vec<channelmonitor::HTLCUpdate> {
		self.monitor.fetch_pending_htlc_updated()
	}
}
