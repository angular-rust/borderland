extern crate borderland;
extern crate conduit_mime_types;
extern crate httparse;
extern crate openssl;

use borderland::{Matcher, Router};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::io::{Read, Write};
use std::net::TcpListener;
// use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

fn respond_landing<T: Read + Write>(mut stream: T) {
    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
    stream.write(response).expect("Write failed");
}

struct StrongMatcher {}

#[allow(dead_code)]
impl StrongMatcher {
    pub fn new() -> StrongMatcher {
        StrongMatcher {}
    }
}

impl Matcher for StrongMatcher {
    fn fit(&self) -> bool {
        true
    }
}

static mut MIME_TYPES: Option<conduit_mime_types::Types> = None;

#[allow(dead_code)]
fn get_mime() -> &'static mut conduit_mime_types::Types {
    unsafe {
        match MIME_TYPES {
            Some(ref mut x) => &mut *x,
            None => panic!(),
        }
    }
}

/**
 * main
 */
fn main() {
    let mut servers = vec![];

    unsafe {
        MIME_TYPES = Some(conduit_mime_types::Types::new().unwrap());
    }

    // let router = router.clone();
    /*
     * HTTP handling - should force redirect to https by design
     */
    servers.push(thread::spawn(move || {
        let falback = Router::new();
        let falback = Arc::new(Mutex::new(falback));

        let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let falback = falback.clone();
                    let remote_addr = stream.peer_addr().unwrap();
                    thread::spawn(move || {
                        // let mime = get_mime();
                        // let mime_type = mime.mime_for_path(Path::new("test.js"));
                        // println!("TYPE {}", mime_type);

                        println!("ACCEPT HTTP. REMOTE {:?}", remote_addr);
                        let falback = falback.lock().unwrap();
                        falback.to_https_scheme(stream, remote_addr);
                    });
                }
                Err(e) => {
                    println!("Connection failed: {:?}", e);
                }
            }
        }
    }));

    /*
     * HTTPS handling
     */
    servers.push(thread::spawn(move || {
        let router = Router::new()
            .options(Box::new(StrongMatcher::new()), respond_landing)
            .options(Box::new(StrongMatcher::new()), respond_landing);

        let router = Arc::new(Mutex::new(router));

        let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        acceptor
            .set_private_key_file("/etc/nginx/ssl/multidomain.key", SslFiletype::PEM)
            .unwrap();
        acceptor
            .set_certificate_chain_file("/etc/nginx/ssl/multidomain.crt")
            .unwrap();
        acceptor.check_private_key().unwrap();
        let acceptor = Arc::new(acceptor.build());

        let listener = TcpListener::bind("0.0.0.0:8443").unwrap();

        for stream in listener.incoming() {
            // blocked
            match stream {
                Ok(stream) => {
                    let remote_addr = stream.peer_addr().unwrap();
                    let acceptor = acceptor.clone();
                    let router = router.clone();

                    thread::spawn(move || {
                        match acceptor.accept(stream) {
                            Ok(stream) => thread::spawn(move || {
                                println!("ACCEPT HTTPS. REMOTE {:?}", remote_addr);
                                let router = router.lock().unwrap();
                                router.handle(stream, remote_addr);
                            }),
                            Err(_e) => thread::spawn(move || {
                                // println!("Connection failed: {:?}", e);
                            }),
                        };
                    });
                }
                Err(_e) => {
                    println!("connection failed");
                }
            }
        }
    }));

    println!("STARTED");
    for server in servers {
        // Wait for the thread to finish. Returns a result.
        let _ = server.join();
    }
}
