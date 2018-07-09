// #[macro_use]
extern crate log;

extern crate env_logger;

extern crate borderland;
extern crate byteorder;
extern crate bytes;
extern crate mio;
extern crate net2;
extern crate slab;

use std::env;
// use std::io;
// use std::io::ErrorKind;
// use std::rc::Rc;

use net2::unix::UnixTcpBuilderExt;
use net2::TcpBuilder;

use mio::net::*;
// use mio::{PollOpt, Ready, Token};

// use bytes::{ByteBuf, MutByteBuf};

use mio::Poll;
use std::net::SocketAddr;

use borderland::*;
use std::fmt::Write;
use std::thread;

// extern crate conduit_mime_types;
// extern crate httparse;
// extern crate mio;
// extern crate openssl;

// extern crate byteorder;
// extern crate slab;

// use std::net::SocketAddr;

// use mio::net::TcpListener;
// use mio::Poll;

// use server::*;

// use borderland::{Matcher, Router, Server};
// // use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
// use std::io::{Read, Write};
// use std::net::SocketAddr;
// // use std::net::TcpListener;
// // use std::path::Path;
// use mio::net::{TcpListener, TcpStream};
// use mio::*;

// use std::sync::{Arc, Mutex};
// use std::thread;

// fn respond_landing<T: Read + Write>(mut stream: T) {
//     let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
//     stream.write(response).expect("Write failed");
// }

// struct StrongMatcher {}

// #[allow(dead_code)]
// impl StrongMatcher {
//     pub fn new() -> StrongMatcher {
//         StrongMatcher {}
//     }
// }

// impl Matcher for StrongMatcher {
//     fn fit(&self) -> bool {
//         true
//     }
// }

// static mut MIME_TYPES: Option<conduit_mime_types::Types> = None;

// #[allow(dead_code)]
// fn get_mime() -> &'static mut conduit_mime_types::Types {
//     unsafe {
//         match MIME_TYPES {
//             Some(ref mut x) => &mut *x,
//             None => panic!(),
//         }
//     }
// }

pub fn write_str(dest: &mut String, s: &str) {
    write!(dest, "{}", s)
        .map_err(|e| panic!("error writing to string: {}", e))
        .unwrap();
}

const HTTP_RES: &'static str = "HTTP/1.1 200 OK
Date: Sun, 22 Nov 2015 01:00:44 GMT
Server: miohack
Connection: $Connection$
Content-Length: $Content-Length$

$Content$";

/**
 * main
 */
fn main() {
    // let mut servers = vec![];

    // unsafe {
    //     MIME_TYPES = Some(conduit_mime_types::Types::new().unwrap());
    // }

    // // let router = router.clone();
    // /*
    //  * HTTP handling - should force redirect to https by design
    //  */
    // servers.push(thread::spawn(move || {
    //     let falback = Router::new();
    //     let falback = Arc::new(Mutex::new(falback));

    //     let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    //     for stream in listener.incoming() {
    //         match stream {
    //             Ok(stream) => {
    //                 let falback = falback.clone();
    //                 let remote_addr = stream.peer_addr().unwrap();
    //                 thread::spawn(move || {
    //                     // let mime = get_mime();
    //                     // let mime_type = mime.mime_for_path(Path::new("test.js"));
    //                     // println!("TYPE {}", mime_type);

    //                     println!("ACCEPT HTTP. REMOTE {:?}", remote_addr);
    //                     let falback = falback.lock().unwrap();
    //                     falback.to_https_scheme(stream, remote_addr);
    //                 });
    //             }
    //             Err(e) => {
    //                 println!("Connection failed: {:?}", e);
    //             }
    //         }
    //     }
    // }));

    /*
     * HTTPS handling
     */
    // servers.push(thread::spawn(move || {
    //     let router = Router::new()
    //         .options(Box::new(StrongMatcher::new()), respond_landing)
    //         .options(Box::new(StrongMatcher::new()), respond_landing);

    //     let router = Arc::new(Mutex::new(router));

    //     let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    //     acceptor
    //         .set_private_key_file("/etc/nginx/ssl/multidomain.key", SslFiletype::PEM)
    //         .unwrap();
    //     acceptor
    //         .set_certificate_chain_file("/etc/nginx/ssl/multidomain.crt")
    //         .unwrap();
    //     acceptor.check_private_key().unwrap();

    //     let acceptor = Arc::new(acceptor.build());

    //     let listener = TcpListener::bind("0.0.0.0:8443").unwrap();

    //     for stream in listener.incoming() {
    //         // blocked
    //         match stream {
    //             Ok(stream) => {
    //                 let remote_addr = stream.peer_addr().unwrap();
    //                 let acceptor = acceptor.clone();
    //                 let router = router.clone();

    //                 thread::spawn(move || {
    //                     match acceptor.accept(stream) {
    //                         Ok(stream) => thread::spawn(move || {
    //                             println!("ACCEPT HTTPS. REMOTE {:?}", remote_addr);
    //                             let router = router.lock();
    //                             match router {
    //                                 Ok(router) => {
    //                                     println!("HERE");
    //                                     router.handle(stream, remote_addr)
    //                                 }
    //                                 Err(e) => println!("lock failed {}", e),
    //                             }
    //                         }),
    //                         Err(e) => thread::spawn(move || {
    //                             println!("Connection failed: {:?}", e);
    //                         }),
    //                     };
    //                 });
    //             }
    //             Err(e) => {
    //                 println!("connection failed {}", e);
    //             }
    //         }
    //     }
    // }));

    // println!("STARTED");
    // for server in servers {
    //     // Wait for the thread to finish. Returns a result.
    //     let _ = server.join();
    // }

    // Before doing anything, let us register a logger. The mio library has really good logging
    // at the _trace_ and _debug_ levels. Having a logger setup is invaluable when trying to
    // figure out why something is not working correctly.
    env_logger::init();

    // let use_st: u32 = env::var("ST").unwrap_or("0".to_string()).parse().unwrap();

    let res_size = env::var("RES_SIZE")
        .unwrap_or("0".to_string())
        .parse()
        .unwrap();

    let content = if res_size == 0 {
        String::from("Have a nice day.")
    } else {
        let res_base = String::from("DEADBEEF");
        let res_base_len = res_base.len();
        let mut written = 0;
        let mut content = String::new();
        while written < res_size {
            write_str(&mut content, &res_base);
            written = written + res_base_len
        }
        content
    };

    let cl = content.to_owned().into_bytes().len();
    println!("response body size: {} bytes", cl);

    let res = String::from(HTTP_RES)
        .replace("$Connection$", "Keep-Alive")
        .replace("$Content$", &content)
        .replace("$Content-Length$", &cl.to_string());

    let res_bytes = res.to_owned().into_bytes();

    println!(
        "full response size (including headers): {}",
        res_bytes.len()
    );
    // println!("buffer capacity: {}", READ_WRITE_BUF_CAP);
    // let res_f = res_bytes.len() as f32 / READ_WRITE_BUF_CAP as f32;
    // println!("expected number of writes per request: {}", res_f.ceil());

    // let res = ResponseData { data: res_bytes };

    let threads = env::var("THREADS")
        .unwrap_or("2".to_string())
        .parse()
        .unwrap();
    println!("multi-threaded server starting: {} threads", threads);
    let mut children = Vec::new();

    let addr = "127.0.0.1:8000"
        .parse::<SocketAddr>()
        .expect("Failed to parse host:port string");

    let tcp = TcpBuilder::new_v4().unwrap();
    tcp.reuse_address(true).unwrap();
    tcp.reuse_port(true).unwrap();
    tcp.bind(addr).unwrap();

    println!("LISTEN {:?}", addr);

    let listener = tcp.listen(4096).unwrap();
    let listener = TcpListener::from_std(listener).unwrap();
    //     let sock = TcpListener::bind(&addr).expect("Failed to bind address");

    for i in 0..threads {
        let listener = listener.try_clone().unwrap();
        // let res = res.clone();

        children.push(thread::spawn(move || {
            // let srv = listener;
            // let res = Rc::from(res);
            let mut poll = Poll::new().expect("Failed to create Poll");

            println!("thread {} accepting connections", i);

            // Create our Server object and start polling for events. I am hiding away
            // the details of how registering works inside of the `Server` object. One reason I
            // really like this is to get around having to have `const SERVER = Token(0)` at the top of my
            // file. It also keeps our polling options inside `Server`.
            let mut server = Server::new(listener);
            server.run(&mut poll).expect("Failed to run server");
        }));
    }

    for child in children {
        child.join().unwrap();
    }
    println!("joined");
}
