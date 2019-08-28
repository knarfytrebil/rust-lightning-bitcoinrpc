use bytes::BufMut;

use futures::future;
use futures::{FutureExt, StreamExt, SinkExt};
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
use std::time::{Duration};
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
        this: Arc<Mutex<Self>>,
        reader: futures::stream::SplitStream<tokio_codec::Framed<TcpStream, tokio_codec::BytesCodec>>,
        larva: T
    ) {
        let this_ref = this.clone();
        let this_close_ref = this.clone();
        let peer_manager_ref = peer_manager.clone();
        let larva_ref = larva.clone();
        let _ = larva.clone().spawn_task(
            async move {
                reader.for_each(move |b| {
                    let b = b.unwrap();
                    let pending_read = b.to_vec();

                    {
                        let mut lock = this_ref.lock().unwrap();
                        assert!(lock.pending_read.is_empty());
                        if lock.read_paused {
                            debug!("READ PAUSED");
                            lock.pending_read = pending_read;
                            let (sender, blocker) = futures::channel::oneshot::channel();
                            lock.read_blocker = Some(sender);
                            return future::Either::Left(blocker.then(|_| { future::ready(()) }));
                        }
                    }

                    //TODO: There's a race where we don't meet the requirements of disconnect_socket if its
                    //called right here, after we release the us_ref lock in the scope above, but before we
                    //call read_event!
                    let mut sd = SocketDescriptor::new(
                        this_ref.clone(),
                        peer_manager.clone(),
                        larva.clone()
                    );
                    match peer_manager.read_event(&mut sd, pending_read) {
                        Ok(pause_read) => {
                            if pause_read {
                                let mut lock = this_ref.lock().unwrap();
                                lock.read_paused = true;
                            }
                        },
                        Err(e) => {
                            error!("Peer Manager Read Error{}", e);
                            this_ref.lock().unwrap().need_disconnect = false;
                            // return future::Either::Right(future::ok(std::io::Error::new(std::io::ErrorKind::InvalidData, e)));
                            // TODO: should discuss Err
                            return future::Either::Right(future::ready(()));
                        }
                    }

                    if let Err(e) = this_ref.lock().unwrap().event_notify.try_send(()) {
                        // Ignore full errors as we just need them to poll after this point, so if the user
                        // hasn't received the last send yet, it doesn't matter.
                        assert!(e.is_full());
                    }


                    // TODO: FYI, this part should be rewrote
                    return future::Either::Right(future::ready(()));

                })
                .then(move |_| {
                    if this_close_ref.lock().unwrap().need_disconnect {
                        peer_manager_ref.disconnect_event(
                            &SocketDescriptor::new(
                                this_close_ref, peer_manager_ref.clone(),
                                larva_ref.clone()
                            )
                        );
                        info!("Peer Disconnected ...");
                    } else {
                        debug!("We disconnected peer!");
                    }
                    future::ok(())
                }).await
            }
        );
    }

    fn new(event_notify: mpsc::Sender<()>, stream: TcpStream, larva: &impl Larva) ->
        (futures::stream::SplitStream<tokio_codec::Framed<TcpStream, tokio_codec::BytesCodec>>, Arc<Mutex<Self>>) {
            let (mut writer, reader) = tokio_codec::Framed::new(stream, tokio_codec::BytesCodec::new()).split();
            let (send_sink, mut send_stream) = mpsc::channel(3);
            let _ = larva.spawn_task(async move {
                let _ = writer.send_all(
                    &mut send_stream
                    // TODO: error handle
                    //     .map_err(|e| -> std::io::Error {
                    //     unreachable!();
                    // })
                ).await;
                Ok(())
                // .and_then(|_| { future::ok(())});
                // Ok(())
            });
            let this = Arc::new(
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
            (reader, this)
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
        larva: T
    ) {
        let (reader, this) = Self::new(event_notify, stream, &larva);

        if let Ok(_) = peer_manager.new_inbound_connection(SocketDescriptor::new(this.clone(), peer_manager.clone(), larva.clone())) {
            Self::schedule_read(peer_manager, this, reader, larva);
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
                .send_data(&initial_send, true) == initial_send.len() {

                    Self::schedule_read(peer_manager, us, reader, larva);
                    info!("Outbound Connection Established {}", &their_node_id);
                } else {
                    debug!("Failed to write first full message to socket!");
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
        addr: SocketAddr, larva: T) where T: Unpin {
        let connect_timeout = futures_timer::Delay::new(Duration::from_secs(10)).then(|_| {
            future::ready(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout reached"))
        });
        let _ = larva.clone().spawn_task(futures::future::select(
            TcpStream::connect(&addr), connect_timeout).then(move |either| {
                match either {
                    future::Either::Left((x, _)) => {
                        if let Ok(stream) = x {
                            Connection::setup_outbound(peer_manager, event_notify, their_node_id, stream, larva);
                            return future::ok(());
                        } else {
                            // TODO show err
                            return future::err(());
                        }
                    },
                    future::Either::Right((_, left)) => {
                        let _ = left.map(|stream| {
                            let stream = stream.unwrap();
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                        });
                        future::err(())
                    },
                }
            }));
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
        debug!("socket descriptior");
        let id = match conn.try_lock() {
            Ok(conn) => {
                debug!("conn {}", &conn.id);
                conn.id
            }
            Err(e) => {
                debug!("About to Panic !!!");
                debug!("{}", e); 
                panic!("{}", e); 
            }
        };
        // let id = conn.lock().unwrap().id;
        debug!("socket id {}", &id);
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
    fn send_data(&mut self, data: &[u8], resume_read: bool) -> usize {
        let mut us = self.conn.lock().unwrap();
        if resume_read {
            let us_ref = self.clone();
            schedule_read!(us_ref);
        }
        if us.writer.is_none() {
            us.read_paused = true;
            return 0;
        }

        let mut bytes = bytes::BytesMut::with_capacity(data.len());

        bytes.put(&data[..]);
        // TODO AsyncSink
        let writer = us.writer.as_ref().unwrap();
        // TODO check logic
        match writer.clone().try_send(bytes.freeze()) {
            Ok(_) => {
                data.len()
            },
            Err(e) => {
                error!("{:?}", e);
                us.read_paused = true;
                let us_ref = self.clone();
                let mut w = us.writer.take().unwrap();
                let _ = self.larva.spawn_task(async move {
                    w.flush().then(move |writer_res| {
                        if let Ok(_) = writer_res {
                            // {
                            //     let mut us = us_ref.conn.lock().unwrap();
                            //     us.writer = Some(writer);
                            // }
                            schedule_read!(us_ref);
                        } // we'll fire the disconnect event on the socket reader end
                        // Ok(())
                        future::ok(())
                    }).await
                });
                0
            },
        }
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
