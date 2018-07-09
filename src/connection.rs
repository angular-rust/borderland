extern crate httparse;

use std::collections::VecDeque;
use std::io;
// use std::io::prelude::*;
use openssl::ssl::SslAcceptor;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::rc::Rc;
use std::sync::Arc;

// use byteorder::{BigEndian, ByteOrder};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use mio::net::TcpStream;
use mio::unix::UnixReady;
use mio::{Poll, PollOpt, Ready, Token};
use std::fmt;
use std::net::Shutdown;
use std::str;
use std::thread;

const READ_WRITE_BUF_CAP: usize = 65536;

#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Proto {
    NONE = 1,
    HTTP,
    HTTPS,
    HTTP2,
    WS,
    WSS,
}

impl fmt::Display for Proto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match *self {
            Proto::HTTP => "HTTP",
            Proto::HTTPS => "HTTPS",
            Proto::HTTP2 => "HTTP2",
            Proto::WS => "WS",
            Proto::WSS => "WSS",
            _ => "NONE",
        };
        write!(f, "{}", printable)
    }
}

/// A stateful wrapper around a non-blocking stream. This connection is not
/// the SERVER connection. This connection represents the client connections
/// _accepted_ by the SERVER connection.
pub struct Connection {
    // handle to the accepted socket
    sock: TcpStream,

    // token used to register with the poller
    pub token: Token,

    // set of events we are interested in
    interest: Ready,

    buf: Option<Bytes>,
    mut_buf: Option<BytesMut>,

    // messages waiting to be sent out
    send_queue: VecDeque<Rc<Vec<u8>>>,

    // track whether a read received `WouldBlock` and store the number of
    // byte we are supposed to read
    // read_continuation: Option<u64>,

    // track whether a write received `WouldBlock`
    write_continuation: bool,

    // keep the protocol used in connection
    proto: Proto,
}

const MAGIC_METHODS: [&str; 9] = [
    "OPT", "GET", "POS", "PUT", "DEL", "HEA", "TRA", "CON", "PAT",
]; // names: [&str; 3]

struct SocketWrapper<T> {
    inner: T,
}

impl<T: Read + Write> SocketWrapper<T> {
    pub fn new(inner: T) -> SocketWrapper<T> {
        SocketWrapper { inner: inner }
    }
}

impl<T: Read + Write> Read for SocketWrapper<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<T: Read + Write> Write for SocketWrapper<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<T: Read + Write> fmt::Debug for SocketWrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GET")
    }
}

impl Connection {
    pub fn new(sock: TcpStream, token: Token) -> Connection {
        Connection {
            sock: sock,
            token: token,
            interest: Ready::from(UnixReady::hup()),
            buf: None,
            mut_buf: Some(BytesMut::with_capacity(READ_WRITE_BUF_CAP)),
            send_queue: VecDeque::with_capacity(32),
            // read_continuation: None,
            write_continuation: false,
            proto: Proto::NONE,
        }
    }

    pub fn hup(&mut self, poll: &mut Poll, dereg: bool) -> io::Result<()> {
        match self.sock.shutdown(Shutdown::Both) {
            Ok(_) => debug!("SHUTDOWN CONNECTION"),
            Err(e) => {
                if e.kind() != ErrorKind::NotConnected {
                    panic!("FAILED SHUTDOWN CONNECTION: {}", e)
                }
            }
        };

        if dereg {
            match poll.deregister(&self.sock) {
                Err(e) => debug!("error hanging up sock: {}", e), // TODO: always fails
                Ok(_) => (),
            }
        }

        Ok(())
    }

    fn read_request_head<T: Read>(stream: T) -> Vec<u8> {
        let mut reader = BufReader::new(stream);
        let mut buff = Vec::new();
        let mut read_bytes = reader
            .read_until(b'\n', &mut buff)
            .expect("reading from stream won't fail");

        while read_bytes > 0 {
            read_bytes = reader.read_until(b'\n', &mut buff).unwrap();
            if read_bytes == 2 && &buff[(buff.len() - 2)..] == b"\r\n" {
                break;
            }
        }
        return buff;
    }

    /// Handle read event from poller.
    ///
    /// The Handler must continue calling until None is returned.
    ///
    /// The recieve buffer is sent back to `Server` so the message can be broadcast to all
    /// listening connections.
    pub fn readable(
        &mut self,
        poll: &mut Poll,
        acceptor: &Arc<SslAcceptor>,
    ) -> io::Result<Option<Vec<u8>>> {
        let local_addr = self.sock.local_addr().unwrap();
        let remote_addr = self.sock.peer_addr().unwrap();

        debug!(
            "PROTOCOL {} ADDR {}:{} -> {}",
            self.proto,
            local_addr.ip(),
            local_addr.port(),
            remote_addr,
        );

        let mut buf = self.mut_buf.take().unwrap();

        {
            // UFCS: resolve "multiple applicable items in scope [E0034]" error
            // let sock_ref = <TcpStream as Read>::by_ref(&mut self.sock);

            // Read header
            let mut wrapper = SocketWrapper::new(&mut self.sock);

            let res = match wrapper.read(unsafe { buf.bytes_mut() }) {
                Ok(n) => {
                    // println!("CONN : we read {} bytes!", n);
                    unsafe {
                        buf.advance_mut(n);
                    }

                    // println!("LEN {} {}", buf.remaining_mut(), buf.capacity());

                    // reuse the buffer for the response
                    // self.buf = Some(buf.flip());
                    self.mut_buf = Some(buf); // DV STUB... REMOVE IT

                    self.interest.remove(Ready::readable());
                    self.interest.insert(Ready::writable());
                }
                Err(e) => {
                    println!("not implemented; client err={:?}", e);
                    self.interest.remove(Ready::readable());
                }
            };

            let mut buf = self.mut_buf.take().unwrap();

            // Detect the proto is unsecure by MAGIC three bytes of HTTP proto when connection initiated
            if self.proto == Proto::NONE {
                let is_plain = {
                    let tri = String::from_utf8_lossy(&buf[0..3]);

                    let ret = match MAGIC_METHODS.iter().find(|&&magic| magic == tri) {
                        Some(_) => true,
                        None => false,
                    };
                    ret
                };

                if is_plain {
                    // SHOULD TO CHECK "Connection: Upgrade" for websocket
                    self.proto = Proto::HTTP;
                    let mut reader = BufReader::new(&buf[..]);
                    let mut buff = Vec::new();

                    let mut read_bytes = reader
                        .read_until(b'\n', &mut buff)
                        .expect("reading from stream won't fail");

                    while read_bytes > 0 {
                        read_bytes = reader.read_until(b'\n', &mut buff).unwrap();
                        if read_bytes == 2 && &buff[(buff.len() - 2)..] == b"\r\n" {
                            break;
                        }
                    }

                    // let s = match str::from_utf8(&buff) {
                    //     Ok(v) => v,
                    //     Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                    // };

                    // println!("REQUEST: \n{}", s);

                    let mut headers = [httparse::EMPTY_HEADER; 16];
                    let mut req = httparse::Request::new(&mut headers);
                    let _ = req.parse(&buff);

                    println!("HTTP VERSION HTTP 1.{:?}", req.version.unwrap());

                    let host = match req.headers.iter().find(|&&header| header.name == "Host") {
                        Some(header) => str::from_utf8(header.value).unwrap(),
                        None => "",
                    };

                    // let method = Method::from_str(req.method.unwrap());
                    let path = req.path.unwrap();

                    // println!("HOST {:?} {:?} {}\n", host, method.unwrap(), path);
                    println!("HOST {:?} {}\n", host, path);

                // for (_i, elem) in req.headers.iter_mut().enumerate() {
                //     let s = match str::from_utf8(elem.value) {
                //         Ok(v) => v,
                //         Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                //     };

                //     println!("HEADER {:?} {:?}", elem.name, s)
                // }
                } else {
                    // HANDLE SSL-LIKE PROTO
                    // thread::spawn(move || {
                    match acceptor.accept(wrapper) {
                        Ok(stream) => {
                            println!("GOT ACCEPT FROM HTTPS");
                            // thread::spawn(move || {
                            //     println!("ACCEPT HTTPS. REMOTE {:?}", remote_addr);
                            //     // let router = router.lock();
                            //     // match router {
                            //     //     Ok(router) => {
                            //     //         println!("HERE");
                            //     //         router.handle(stream, remote_addr)
                            //     //     }
                            //     //     Err(e) => println!("lock failed {}", e),
                            //     // }
                            // })
                        }
                        Err(e) => {
                            // thread::spawn(move || {
                            println!("Connection failed: {:?}", e);
                            // })
                        }
                    };
                    // });
                }

                println!("ACHTUNG!!! IS PLAIN {}", is_plain);
            }
        }

        // self.interest.insert(Ready::writable());
        // self.interest.remove(Ready::readable());
        let _ = self.reregister(poll);

        let msg = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
        Ok(Some(msg.to_vec()))
    }

    /// Handle a writable event from the poller.
    ///
    /// Send one message from the send queue to the client. If the queue is empty, remove interest
    /// in write events.
    /// TODO: Figure out if sending more than one message is optimal. Maybe we should be trying to
    /// flush until the kernel sends back EAGAIN?
    pub fn writable(&mut self, poll: &mut Poll) -> io::Result<()> {
        // self.send_queue
        //     .pop_front()
        //     .ok_or(Error::new(ErrorKind::Other, "Could not pop send queue"))
        //     .and_then(|buf| self.write_message(buf))?;

        // if self.send_queue.is_empty() {
        //     self.interest.remove(Ready::writable());
        // }

        self.interest.insert(Ready::readable());
        self.interest.remove(Ready::writable());
        let _ = self.reregister(poll);

        Ok(())
    }

    fn write_message(&mut self, buf: Rc<Vec<u8>>) -> io::Result<()> {
        // println!("MESSAGE {:?}", String::from_utf8_lossy(&buf));

        let len = buf.len();
        match self.sock.write(&*buf) {
            Ok(n) => {
                debug!("CONN : we wrote {} bytes", n);
                // if we wrote a partial message, then put remaining part of message back
                // into the queue so we can try again
                if n < len {
                    let remaining = Rc::new(buf[n..].to_vec());
                    self.send_queue.push_front(remaining);
                    self.write_continuation = true;
                } else {
                    self.write_continuation = false;
                }
                Ok(())
            }
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    debug!("client flushing buf; WouldBlock");

                    // put message back into the queue so we can try again
                    self.send_queue.push_front(buf);
                    self.write_continuation = true;
                    Ok(())
                } else {
                    error!("Failed to send buffer for {:?}, error: {}", self.token, e);
                    Err(e)
                }
            }
        }
    }

    /// Queue an outgoing message to the client.
    ///
    /// This will cause the connection to register interests in write events with the poller.
    /// The connection can still safely have an interest in read events. The read and write buffers
    /// operate independently of each other.
    pub fn send_message(&mut self, message: Rc<Vec<u8>>) -> io::Result<()> {
        trace!("connection send_message; token={:?}", self.token);

        // if the queue is empty then try and write. if we get WouldBlock the message will get
        // queued up for later. if the queue already has items in it, then we know that we got
        // WouldBlock from a previous write, so queue it up and wait for the next write event.
        if self.send_queue.is_empty() {
            self.write_message(message)?;
        } else {
            self.send_queue.push_back(message);
        }

        if !self.send_queue.is_empty() && !self.interest.is_writable() {
            self.interest.insert(Ready::writable());
        }

        Ok(())
    }

    /// Register interest in read events with poll.
    ///
    /// This will let our connection accept reads starting next poller tick.
    pub fn register(&mut self, poll: &mut Poll) -> io::Result<()> {
        trace!("connection register; token={:?}", self.token);

        self.interest.insert(Ready::readable());

        poll.register(
            &self.sock,
            self.token,
            self.interest,
            PollOpt::edge() | PollOpt::oneshot(),
        ).or_else(|e| {
            error!("Failed to reregister {:?}, {:?}", self.token, e);
            Err(e)
        })
    }

    /// Re-register interest in read events with poll.
    pub fn reregister(&mut self, poll: &mut Poll) -> io::Result<()> {
        trace!("connection reregister; token={:?}", self.token);

        poll.reregister(
            &self.sock,
            self.token,
            self.interest,
            PollOpt::edge() | PollOpt::oneshot(),
        ).or_else(|e| {
            error!("Failed to reregister {:?}, {:?}", self.token, e);
            Err(e)
        })
    }
}
