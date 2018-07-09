use mio::net::TcpListener;
use mio::unix::UnixReady;
use mio::{Events, Poll, PollOpt, Ready, Token};

use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use slab;
use std::io::{self, ErrorKind};
use std::rc::Rc;
use std::sync::Arc;

use connection::Connection;

type Slab<T> = slab::Slab<T, Token>;

pub struct Server {
    // main socket for our server
    sock: TcpListener,

    // token of our server. we keep track of it here instead of doing `const SERVER = Token(_)`.
    token: Token,

    // a list of connections _accepted_ by our server
    conns: Slab<Connection>,

    acceptor: Arc<SslAcceptor>,
}

const READ_WRITE_CAP: usize = 65536;

impl Server {
    pub fn new(sock: TcpListener) -> Server {
        let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        acceptor
            .set_private_key_file("/etc/nginx/ssl/multidomain.key", SslFiletype::PEM)
            .unwrap();
        acceptor
            .set_certificate_chain_file("/etc/nginx/ssl/multidomain.crt")
            .unwrap();
        acceptor.check_private_key().unwrap();

        // let acceptor = Arc::new(acceptor.build());

        Server {
            sock: sock,

            // Give our server token a number much larger than our slab capacity. The slab used to
            // track an internal offset, but does not anymore.
            token: Token(10_000_000),

            // We will handle a max of READ_WRITE_CAP connections
            conns: Slab::with_capacity(READ_WRITE_CAP),

            acceptor: Arc::new(acceptor.build()),
        }
    }

    pub fn run(&mut self, poll: &mut Poll) -> io::Result<()> {
        self.register(poll)?;

        info!("SERVER RUN LOOP STARTING...");
        loop {
            let mut events = Events::with_capacity(1024);
            poll.poll(&mut events, None)?;
            for event in events.iter() {
                trace!("EVENT={:?}", event);
                self.ready(poll, event.token(), event.readiness());
            }
        }
    }

    /// Register Server with the poller.
    ///
    /// This keeps the registration details neatly tucked away inside of our implementation.
    pub fn register(&mut self, poll: &mut Poll) -> io::Result<()> {
        poll.register(&self.sock, self.token, Ready::readable(), PollOpt::edge())
            .or_else(|e| {
                error!("Failed to register server {:?}, {:?}", self.token, e);
                Err(e)
            })
    }

    /// Remove a token from the slab
    fn remove_token(&mut self, token: Token) {
        match self.conns.remove(token) {
            Some(_c) => {
                debug!("reset connection; token={:?}", token);
            }
            None => {
                warn!("Unable to remove connection for {:?}", token);
            }
        }
    }

    fn report_slab_size(&mut self) {
        debug!("slab size: {}", self.conns.len());
        // let slsize = self.conns.count();
        // if slsize % 200 == 0 {
        //     println!("now at {} connections", slsize);
        // }
    }

    fn conn_hup(&mut self, poll: &mut Poll, token: Token) -> io::Result<()> {
        debug!("SERVER CONN HUP; tok={:?}", token);
        let res = self.connection(token).hup(poll, true);
        // TODO: hup res ok?
        self.remove_token(token);
        self.report_slab_size();
        res
    }

    fn ready(&mut self, poll: &mut Poll, token: Token, event: Ready) {
        debug!("GOT {:?} EVENT = {:?}", token, event);

        if self.token != token && self.conns.contains(token) == false {
            debug!("Failed to find connection for {:?}", token);
            return;
        }

        let event = UnixReady::from(event);

        if event.is_error() {
            warn!("Error event for {:?}", token);
            self.remove_token(token);
            return;
        }

        if event.is_hup() {
            trace!("HUP EVENT FOR {:?}", token);
            let _ = self.conn_hup(poll, token);
            return;
        }

        let event = Ready::from(event);

        // We never expect a write event for our `Server` token . A write event for any other token
        // should be handed off to that connection.
        if event.is_writable() {
            // println!("### WRITE EVENT FOR {:?}", token);
            assert!(self.token != token, "Received writable event for Server");

            // Forward a readable event to an established connection.
            //
            // Connections are identified by the token provided to us from the poller. Once a read has
            // finished, push the receive buffer into the all the existing connections so we can
            // broadcast.
            match self.connection(token).writable(poll) {
                Ok(()) => {
                    let message = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
                    let rc_message = Rc::new(message.to_vec());
                    self.connection(token)
                        .send_message(rc_message.clone())
                        .unwrap();

                    // while let Some(message) = self.connection(token).readable()? {
                    // let rc_message = Rc::new(message);
                    // self.connection(token).send_message(rc_message.clone())?;
                    // Echo the message too all connected clients.
                    // for c in self.conns.iter_mut() {
                    //     c.send_message(rc_message.clone())?;
                    // }
                    // }

                    // println!("SHOULD HUP FOR HTTP {:?}", token);
                    let _ = self.conn_hup(poll, token);
                    return;
                }
                Err(e) => {
                    warn!("Write event failed for {:?}, {:?}", token, e);
                    self.remove_token(token);
                    return;
                }
            }
        }

        // A read event for our `Server` token means we are establishing a new connection. A read
        // event for any other token should be handed off to that connection.
        if event.is_readable() {
            trace!("### READ EVENT FOR {:?}", token);

            let acc = &mut self.acceptor.clone();

            if self.token == token {
                self.accept(poll);
            } else {
                let conn = self.connection(token);

                match conn.readable(poll, acc).unwrap() {
                    // should return empty message and keep it inside connection to reduce data movement as it unused
                    Some(message) => {
                        // println!("GOT MESSAGE {}", String::from_utf8_lossy(&message));
                        // let rc_message = Rc::new(message);
                        // conn.send_message(rc_message.clone()).unwrap();
                    }
                    None => {}
                }
            }
        }

        if self.token != token {
            match self.connection(token).reregister(poll) {
                Ok(()) => {}
                Err(e) => {
                    warn!("Reregister failed {:?}", e);
                    self.remove_token(token);
                    return;
                }
            }
        }
    }

    /// Accept a _new_ client connection.
    ///
    /// The server will keep track of the new connection and forward any events from the poller
    /// to this connection.
    fn accept(&mut self, poll: &mut Poll) {
        debug!("SERVET ACCEPTING NEW SOCKET");

        loop {
            // Log an error if there is no socket, but otherwise move on so we do not tear down the
            // entire server.
            let sock = match self.sock.accept() {
                Ok((sock, _)) => sock,
                Err(e) => {
                    if e.kind() == ErrorKind::WouldBlock {
                        debug!("accept encountered WouldBlock");
                    } else {
                        error!("Failed to accept new socket, {:?}", e);
                    }
                    return;
                }
            };

            let token = match self.conns.vacant_entry() {
                Some(entry) => {
                    let c = Connection::new(sock, entry.index());
                    entry.insert(c).index()
                }
                None => {
                    error!("Failed to insert connection into slab");
                    return;
                }
            };

            debug!("REGISTERING {:?} WITH POLLER", token);
            match self.connection(token).register(poll) {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        "Failed to register {:?} connection with poller, {:?}",
                        token, e
                    );
                    self.remove_token(token);
                }
            }
        }
    }

    /// Find a connection in the slab using the given token.
    ///
    /// This function will panic if the token does not exist. Use self.conns.contains(token)
    /// before using this function.
    fn connection(&mut self, token: Token) -> &mut Connection {
        &mut self.conns[token]
    }
}
