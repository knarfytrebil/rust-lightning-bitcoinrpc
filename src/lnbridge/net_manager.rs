use bytes;
use bytes::BufMut;

use std;
use std::mem;
use std::sync::{Arc, Mutex};
use std::vec::Vec;
use std::hash::{Hash, Hasher};

use tokio;
use tokio::net::TcpStream;
use tokio_codec::{Framed, BytesCodec};

use secp256k1::key::PublicKey;

use futures;
use futures::future;
use futures::future::{Future, Either};
use futures::stream::SplitStream;
use futures::{AsyncSink, Stream, Sink};
use futures::sync::mpsc;

use lightning::ln::peer_handler::{SocketDescriptor, PeerManager};

pub struct LnPeerConnector {
	writer: Option<mpsc::Sender<bytes::Bytes>>,
	event_notify: mpsc::UnboundedSender<()>,
	pending_read: Vec<u8>,
	read_blocker: Option<futures::sync::oneshot::Sender<Result<(), ()>>>,
	read_paused: bool,
	need_disconnect: bool,
	id: u64,
}

impl LnPeerConnector {
	pub fn new(event_notify: mpsc::UnboundedSender<()>, stream: TcpStream, id: u64) -> (SplitStream<Framed<TcpStream, BytesCodec>>, Arc<Mutex<Self>>) {
		let (writer, reader) = Framed::new(stream, BytesCodec::new()).split();
		let (send_sink, send_stream) = mpsc::channel(3);
		tokio::spawn(writer.send_all(send_stream.map_err(|_| -> std::io::Error {
			unreachable!();
		})).then(|_| {
			future::result(Ok(()))
		}));
		let us = Arc::new(Mutex::new(Self {
      writer: Some(send_sink),
      event_notify,
      pending_read: Vec::new(),
      read_blocker: None,
      read_paused: false,
      need_disconnect: true,
      id
    }));

		(reader, us)
	}
}

pub fn setup_inbound(peer_manager: Arc<PeerManager<LnSocketDescriptor>>, event_notify: mpsc::UnboundedSender<()>, stream: TcpStream, id: u64) {
	let (reader, us) = LnPeerConnector::new(event_notify, stream, id);

	if let Ok(_) = peer_manager.new_inbound_connection(LnSocketDescriptor::new(us.clone(), peer_manager.clone())) {
		schedule_read(peer_manager, us, reader);
	}
}

pub fn setup_outbound(peer_manager: Arc<PeerManager<LnSocketDescriptor>>, event_notify: mpsc::UnboundedSender<()>, their_node_id: PublicKey, stream: TcpStream, id: u64) {
	let (reader, us) = LnPeerConnector::new(event_notify, stream, id);

	if let Ok(initial_send) = peer_manager.new_outbound_connection(their_node_id, LnSocketDescriptor::new(us.clone(), peer_manager.clone())) {
		if LnSocketDescriptor::new(us.clone(), peer_manager.clone()).send_data(&initial_send, 0, true) == initial_send.len() {
			schedule_read(peer_manager, us, reader);
		} else {
			println!("Failed to write first full message to socket!");
		}
	}
}

pub	fn schedule_read(
  peer_manager: Arc<PeerManager<LnSocketDescriptor>>,
  us: Arc<Mutex<LnPeerConnector>>,
  reader: SplitStream<Framed<TcpStream, BytesCodec>>) {
	let us_ref = us.clone();
	let us_close_ref = us.clone();
	let peer_manager_ref = peer_manager.clone();
	tokio::spawn(reader.for_each(move |b| {
		let pending_read = b.to_vec();
		{
			let mut lock = us_ref.lock().unwrap();
			assert!(lock.pending_read.is_empty());
			if lock.read_paused {
				lock.pending_read = pending_read;
				let (sender, blocker) = futures::sync::oneshot::channel();
				lock.read_blocker = Some(sender);
				return Either::A(blocker.then(|_| { Ok(()) }));
			}
		}
		//TODO: There's a race where we don't meet the requirements of disconnect_socket if its
		//called right here, after we release the us_ref lock in the scope above, but before we
		//call read_event!
		match peer_manager.read_event(&mut LnSocketDescriptor::new(us_ref.clone(), peer_manager.clone()), pending_read) {
			Ok(pause_read) => {
				if pause_read {
					let mut lock = us_ref.lock().unwrap();
					lock.read_paused = true;
				}
			},
			Err(e) => {
				us_ref.lock().unwrap().need_disconnect = false;
				return Either::B(future::result(Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e))));
			}
		}

		us_ref.lock().unwrap().event_notify.unbounded_send(()).unwrap();

		Either::B(future::result(Ok(())))
	}).then(move |_| {
		if us_close_ref.lock().unwrap().need_disconnect {
			peer_manager_ref.disconnect_event(&LnSocketDescriptor::new(us_close_ref, peer_manager_ref.clone()));
			println!("Peer disconnected!");
		} else {
			println!("We disconnected peer!");
		}
		Ok(())
	}));
}

#[derive(Clone)]
pub struct LnSocketDescriptor {
	connector: Arc<Mutex<LnPeerConnector>>,
	id: u64,
	peer_manager: Arc<PeerManager<LnSocketDescriptor>>,
}
impl LnSocketDescriptor {
	pub fn new(
    connector: Arc<Mutex<LnPeerConnector>>,
    peer_manager: Arc<PeerManager<LnSocketDescriptor>>
  ) -> Self {
		let id = connector.lock().unwrap().id;
		Self {
      connector,
      id,
      peer_manager
    }
	}
}
impl SocketDescriptor for LnSocketDescriptor {
	fn send_data(&mut self, data: &Vec<u8>, write_offset: usize, resume_read: bool) -> usize {
		macro_rules! schedule_read {
			($us_ref: expr) => {
				tokio::spawn(future::lazy(move || -> Result<(), ()> {
					let mut read_data = Vec::new();
					{
						let mut us = $us_ref.connector.lock().unwrap();
            let pending = us.pending_read.clone();
						mem::swap(&mut read_data, &mut us.pending_read);
					}
					if !read_data.is_empty() {
						let mut us_clone = $us_ref.clone();
						match $us_ref.peer_manager.read_event(&mut us_clone, read_data) {
							Ok(pause_read) => {
								if pause_read { return Ok(()); }
							},
							Err(_) => {
								//TODO: Not actually sure how to do this
								return Ok(());
							}
						}
					}
					let mut us = $us_ref.connector.lock().unwrap();
					if let Some(sender) = us.read_blocker.take() {
						sender.send(Ok(())).unwrap();
					}
					us.read_paused = false;
					us.event_notify.unbounded_send(()).unwrap();
					Ok(())
				}));
			}
		}

		let mut us = self.connector.lock().unwrap();
    //
		if resume_read {
			let us_ref = self.clone();
			schedule_read!(us_ref);
		}
    // 
		if data.len() == write_offset { return 0; }
		if us.writer.is_none() {
			us.read_paused = true;
			return 0;
		}

		let mut bytes = bytes::BytesMut::with_capacity(data.len() - write_offset);
		bytes.put(&data[write_offset..]);
		let write_res = us.writer.as_mut().unwrap().start_send(bytes.freeze());
    println!("write res, offset: {}", write_offset);
    for k in &data[write_offset..] {
      print!("{}", k);
    }
    println!("");
		match write_res {
			Ok(res) => {
				match res {
					AsyncSink::Ready => {
						data.len() - write_offset
					},
					AsyncSink::NotReady(_) => {
						us.read_paused = true;
						let us_ref = self.clone();
						tokio::spawn(us.writer.take().unwrap().flush().then(move |writer_res| -> Result<(), ()> {
							if let Ok(writer) = writer_res {
								{
									let mut us = us_ref.connector.lock().unwrap();
									us.writer = Some(writer);
								}
								schedule_read!(us_ref);
							} // we'll fire the disconnect event on the socket reader end
							Ok(())
						}));
						0
					}
				}
			},
			Err(_) => {
				// We'll fire the disconnected event on the socket reader end
				0
			},
		}
	}

	fn disconnect_socket(&mut self) {
		let mut us = self.connector.lock().unwrap();
		us.need_disconnect = true;
		us.read_paused = true;
	}
}
impl Eq for LnSocketDescriptor {}
impl PartialEq for LnSocketDescriptor {
	fn eq(&self, o: &Self) -> bool {
		self.id == o.id
	}
}
impl Hash for LnSocketDescriptor {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id.hash(state);
	}
}

