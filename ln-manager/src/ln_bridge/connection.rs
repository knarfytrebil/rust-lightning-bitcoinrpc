use bytes::BufMut;

use futures::future;
use futures::{Sink, Poll};
use futures::{FutureExt, StreamExt, SinkExt, TryStreamExt, TryFutureExt};
// use futures::task::Poll;
use futures::channel::mpsc;

use secp256k1::key::PublicKey;

// use tokio::timer::Delay;
use tokio_tcp::TcpStream;

use lightning::ln::peer_handler;
use lightning::ln::peer_handler::SocketDescriptor as LnSocketTrait;

use std::mem;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::vec::Vec;
use std::hash::Hash;

use crate::executor::Larva;

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A connection to a remote peer. Can be constructed either as a remote connection using
/// Connection::setup_outbound o
pub struct Connection {
    writer: Option<mpsc::Sender<bytes::Bytes>>,
    event_notify: mpsc::Sender<()>,
    pending_read: Vec<u8>,
    read_blocker: Option<futures::channel::oneshot::Sender<Result<(), ()>>>,
    read_paused: bool,
    need_disconnect: bool,
    id: u64,
}

impl Connection {
    fn schedule_read<T: Larva>(
        peer_manager: Arc<peer_handler::PeerManager<SocketDescriptor<T>>>,
        us: Arc<Mutex<Self>>,
        reader: futures::stream::SplitStream<tokio_codec::Framed<TcpStream, tokio_codec::BytesCodec>>,
        larva: T
    ) {
        let us_ref = us.clone();
        let us_close_ref = us.clone();
        let peer_manager_ref = peer_manager.clone();
        let larva_ref = larva.clone();
        let _ = larva.clone().spawn_task(
            reader.for_each(move |b| {
                let b = b.unwrap();
                let pending_read = b.to_vec();
                let mut lock = us_ref.lock().unwrap();
                assert!(lock.pending_read.is_empty());

                if lock.read_paused {
                    lock.pending_read = pending_read;
                    let (sender, blocker) = futures::channel::oneshot::channel();
                    lock.read_blocker = Some(sender);
                    return future::Either::Left(blocker.then(|_| { future::ready(()) }));
                }

                //TODO: There's a race where we don't meet the requirements of disconnect_socket if its
                //called right here, after we release the us_ref lock in the scope above, but before we
                //call read_event!
                match peer_manager.read_event(
                    &mut SocketDescriptor::new(
                        us_ref.clone(),
                        peer_manager.clone(),
                        larva.clone()
                    ), pending_read) {
                    Ok(pause_read) => {
                        if pause_read {
                            let mut lock = us_ref.lock().unwrap();
                            lock.read_paused = true;
                        }
                    },
                    Err(e) => {
                        us_ref.lock().unwrap().need_disconnect = false;
                        // return future::Either::Right(future::ok(std::io::Error::new(std::io::ErrorKind::InvalidData, e)));
                        // TODO: should discuss Err
                        return future::Either::Right(future::ready(()));
                    }
                }

                if let Err(e) = us_ref.lock().unwrap().event_notify.try_send(()) {
                    // Ignore full errors as we just need them to poll after this point, so if the user
                    // hasn't received the last send yet, it doesn't matter.
                    assert!(e.is_full());
                }

                // TODO: FYI, this part should be rewrote
                return future::Either::Right(future::ready(()));

            })
                .then(move |_| {
                    if us_close_ref.lock().unwrap().need_disconnect {
                        peer_manager_ref.disconnect_event(
                            &SocketDescriptor::new(
                                us_close_ref, peer_manager_ref.clone(),
                                larva_ref.clone()
                            )
                        );
                        println!("Peer disconnected!");
                    } else {
                        println!("We disconnected peer!");
                    }
                    future::ok(())
                })
        );
    }

    fn new(event_notify: mpsc::Sender<()>, stream: TcpStream, larva: &impl Larva) ->
        (futures::stream::SplitStream<tokio_codec::Framed<TcpStream, tokio_codec::BytesCodec>>, Arc<Mutex<Self>>) {
            let (mut writer, reader) = tokio_codec::Framed::new(stream, tokio_codec::BytesCodec::new()).split();
            let (send_sink, mut send_stream) = mpsc::channel(3);
            // TODO: error handle
            let _ = larva.spawn_task(async move {
                writer.send_all(
                    &mut send_stream
                    //     .map_err(|e| -> std::io::Error {
                    //         unreachable!();
                    //     // std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    //     // std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    // })
                ).and_then(|_| { future::ok(())});
                Ok(())
            });
            let us = Arc::new(
                Mutex::new(
                    Self {
                        writer: Some(send_sink),
                        event_notify,
                        pending_read: Vec::new(),
                        read_blocker: None,
                        read_paused: false,
                        need_disconnect: true,
                        id: ID_COUNTER.fetch_add(1, Ordering::AcqRel)
                    }
                )
            );
            (reader, us)
        }

    /// Process incoming messages and feed outgoing messages on the provided socket generated by
    /// accepting an incoming connection (by scheduling futures with tokio::spawn).
    ///
    /// You should poll the Receive end of event_notify and call get_and_clear_pending_events() on
    /// ChannelManager and ChannelMonitor objects.
    pub fn setup_inbound<T: Larva>(
        peer_manager: Arc<peer_handler::PeerManager<SocketDescriptor<T>>>,
        event_notify: mpsc::Sender<()>,
        stream: TcpStream,
        larva: T) {
        let (reader, us) = Self::new(event_notify, stream, &larva);

        if let Ok(_) = peer_manager.new_inbound_connection(SocketDescriptor::new(us.clone(), peer_manager.clone(), larva.clone())) {
            Self::schedule_read(peer_manager, us, reader, larva);
        }
    }

    /// Process incoming messages and feed outgoing messages on the provided socket generated by
    /// making an outbound connection which is expected to be accepted by a peer with the given
    /// public key (by scheduling futures with tokio::spawn).
    ///
    /// You should poll the Receive end of event_notify and call get_and_clear_pending_events() on
    /// ChannelManager and ChannelMonitor objects.
    pub fn setup_outbound<T: Larva> (
        peer_manager: Arc<peer_handler::PeerManager<SocketDescriptor<T>>>,
        event_notify: mpsc::Sender<()>,
        their_node_id: PublicKey,
        stream: TcpStream, larva: T) {
        let (reader, us) = Self::new(event_notify, stream, &larva);
        if let Ok(initial_send) = peer_manager.new_outbound_connection(
            their_node_id,
            SocketDescriptor::new(us.clone(), peer_manager.clone(), larva.clone())
        ) {
            if SocketDescriptor::new(us.clone(), peer_manager.clone(), larva.clone())
                .send_data(&initial_send, 0, true) == initial_send.len() {

                    Self::schedule_read(peer_manager, us, reader, larva);
                } else {
                    println!("Failed to write first full message to socket!");
                }
        }
    }

    /// Process incoming messages and feed outgoing messages on a new connection made to the given
    /// socket address which is expected to be accepted by a peer with the given public key (by
    /// scheduling futures with tokio::spawn).
    ///
    /// You should poll the Receive end of event_notify and call get_and_clear_pending_events() on
    /// ChannelManager and ChannelMonitor objects.
    pub fn connect_outbound<T: Larva>(
        peer_manager: Arc<peer_handler::PeerManager<SocketDescriptor<T>>>,
        event_notify: mpsc::Sender<()>,
        their_node_id: PublicKey,
        addr: SocketAddr, larva: T) {
        // TODO: discuss
        let connect_timeout = tokio_timer::Delay::new(Instant::now() + Duration::from_secs(10)).then(|_| {
            future::ready(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout reached"))
        });
        // larva.clone().spawn_task(futures::future::select(
        //     TcpStream::connect(&addr).then(move |stream| {
        //         // let stream = stream.unwrap();
        //         // Connection::setup_outbound(peer_manager, event_notify, their_node_id, stream, larva);
        //         future::ok(())
        //     }), connect_timeout).then(|either| {
        //         // match either {
        //         //     future::Either::Left(_) => future::ok(()),
        //         //     future::Either::Right(_) => future::err(()),
        //         // }
        //     }));
        // TODO: consider lifetime
        // let _ = larva.clone().spawn_task(TcpStream::connect(&addr).select(connect_timeout)
        // .and_then(move |stream| {
        //     Connection::setup_outbound(peer_manager, event_notify, their_node_id, stream.0, larva);
        //     future::ok(())
        // }).or_else(|_| {
        //     //TODO: return errors somehow
        //     future::ok(())
        // }));
    }
}

#[derive(Clone)]
pub struct SocketDescriptor<T: Larva> {
    conn: Arc<Mutex<Connection>>,
    id: u64,
    peer_manager: Arc<peer_handler::PeerManager<SocketDescriptor<T>>>,
    larva: T,
}

impl<T: Larva> SocketDescriptor<T> {
    fn new(
        conn: Arc<Mutex<Connection>>,
        peer_manager: Arc<peer_handler::PeerManager<SocketDescriptor<T>>>,
        larva: T,
    ) -> Self {
        let id = conn.lock().unwrap().id;
        Self { conn, id, peer_manager, larva }
    }
}

macro_rules! schedule_read {
    ($us_ref: expr) => {
        let _ = $us_ref.clone().larva.spawn_task(
            future::lazy(move |_| -> Result<(), ()> {
                let mut read_data = Vec::new();
                {
                    let mut us = $us_ref.conn.lock().unwrap();
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
                let mut us = $us_ref.conn.lock().unwrap();
                if let Some(sender) = us.read_blocker.take() {
                    sender.send(Ok(())).unwrap();
                }
                us.read_paused = false;
                if let Err(e) = us.event_notify.try_send(()) {
                    // Ignore full errors as we just need them to poll after this point, so if the user
                    // hasn't received the last send yet, it doesn't matter.
                    assert!(e.is_full());
                }
                Ok(())
            })
        );
    }
}

impl<T: Larva> peer_handler::SocketDescriptor for SocketDescriptor<T> {
    fn send_data(&mut self, data: &Vec<u8>, write_offset: usize, resume_read: bool) -> usize {
        let mut us = self.conn.lock().unwrap();
        if resume_read {
            let us_ref = self.clone();
            schedule_read!(us_ref);
        }
        if data.len() == write_offset { return 0; }
        if us.writer.is_none() {
            us.read_paused = true;
            return 0;
        }

        let mut bytes = bytes::BytesMut::with_capacity(data.len() - write_offset);
        bytes.put(&data[write_offset..]);
        // TODO AsyncSink
		    // let write_res = us.writer.unwrap().start_send(bytes.freeze());
		    // match write_res {
			  //     Ok(res) => {
        //         // TODO: wtf
				//         // match res {
				// 	      //     Poll::Pending => {
				// 		    //         data.len() - write_offset
				//         //     },
				//         //     Poll::Ready(_) => {
				//         //         us.read_paused = true;
				//         //         let us_ref = self.clone();
				//         //         let _ = self.larva.spawn_task(us.writer.take().unwrap().flush().then(move |writer_res| -> Result<(), ()> {
				// 		    //             if let Ok(writer) = writer_res {
				// 				//                 {
				// 				// 	                  let mut us = us_ref.conn.lock().unwrap();
				// 				// 	                  us.writer = Some(writer);
				// 				//                 }
				// 				//                 schedule_read!(us_ref);
				// 		    //             } // we'll fire the disconnect event on the socket reader end
				// 		    //             Ok(())
				//         //         }));
		    //         //         0
        //         //     }
				//         // }
        //         data.len() - write_offset
			  //     },
			  //     Err(_) => {
				//         // We'll fire the disconnected event on the socket reader end
				//         0
			  //     },
		    // }
        0
	  }

    fn disconnect_socket(&mut self) {
        let mut us = self.conn.lock().unwrap();
        us.need_disconnect = true;
        us.read_paused = true;
    }
}

impl<T: Larva> Eq for SocketDescriptor<T> {}
impl<T: Larva> PartialEq for SocketDescriptor<T> {
    fn eq(&self, o: &Self) -> bool {
    self.id == o.id
}
}

impl<T: Larva> Hash for SocketDescriptor<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
