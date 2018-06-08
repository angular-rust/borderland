extern crate borderland;
extern crate conduit_mime_types;
extern crate httparse;
extern crate openssl;

use borderland::Method;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod /*, SslStream*/};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener /*, TcpStream*/};
use std::path::Path;
use std::str;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;

static mut MIME_TYPES: Option<conduit_mime_types::Types> = None;

pub fn get_mime() -> &'static mut conduit_mime_types::Types {
    unsafe {
        match MIME_TYPES {
            Some(ref mut x) => &mut *x,
            None => panic!(),
        }
    }
}

fn respond_hello_world<W: Write>(mut stream: W) {
    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
    stream.write(response).expect("Write failed");
}

fn serve_static_file<W: Write>(mut stream: W, path: &str) {
    println!("STATIC {:?}", format!("www/{}", path));
    let mime_type = get_mime().mime_for_path(Path::new(path));
    println!("MIME {:?}", mime_type);
    let mut file = match File::open(format!("www/{}", path)) {
        Ok(file) => file,
        Err(_) => File::open("404.html").expect("404.html file missing!"),
    };

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Read failed");

    println!("\nPATH: {} LENGTH: {}\n", path, buffer.len());

    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: {}\r\n\r\n", mime_type);
    stream.write(response.as_bytes()).expect("Write failed");
    stream.write_all(&buffer).expect("Write failed");
}

fn respond_error<W: Write>(mut stream: W) {
    let response = b"HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>500 - Server Error</body></html>\r\n";
    stream.write(response).expect("Write failed");
}

fn respond_file_not_found<W: Write>(mut stream: W) {
    let response = b"HTTP/1.1 404 File Not Found\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>404 - File Not Found</body></html>\r\n";
    stream.write(response).expect("Write failed");
}

fn read_request_head<T: Read>(stream: T) -> Vec<u8> {
    let mut reader = BufReader::new(stream);
    let mut buff = Vec::new();
    let mut read_bytes = reader.read_until(b'\n', &mut buff).unwrap();
    while read_bytes > 0 {
        read_bytes = reader.read_until(b'\n', &mut buff).unwrap();
        if read_bytes == 2 && &buff[(buff.len() - 2)..] == b"\r\n" {
            break;
        }
    }
    return buff;
}

fn handle_http_scheme<RW: Read + Write>(mut stream: RW, _client_addr: SocketAddr) {
    let request_bytes = read_request_head(Read::by_ref(&mut stream));
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);
    let _ = req.parse(&request_bytes);

    println!("HTTP VERSION HTTP 1.{:?}", req.version.unwrap());

    let host = match req.headers.iter().find(|&&header| header.name == "Host") {
        Some(header) => str::from_utf8(header.value).unwrap(),
        None => "",
    };

    let method = Method::from_str(req.method.unwrap());
    let path = req.path.unwrap();

    println!("HOST {:?} {:?} {}\n", host, method.unwrap(), path);

    let response = format!(
        "HTTP/1.1 301 Moved Permanently\r\nLocation: https://{}{}\r\n\r\n",
        "localhost:8443", path
    );
    println!("{}", response);
    stream.write(response.as_bytes()).expect("Write failed");
}

/**
 * routing
 */
fn handle_request<RW: Read + Write>(mut stream: RW, _client_addr: SocketAddr) {
    let request_bytes = read_request_head(Read::by_ref(&mut stream));
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);
    let _ = req.parse(&request_bytes);

    println!("VERSION {:?}", req.version.unwrap());
    let method = Method::from_str(req.method.unwrap());
    println!("METHOD {:?}", method.unwrap());
    println!("PATH {:?}\n", req.path.unwrap());

    for (_i, elem) in req.headers.iter_mut().enumerate() {
        let s = match str::from_utf8(elem.value) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };

        println!("{:?} {:?}", elem.name, s)
    }

    let _body_length: u32 = match req
        .headers
        .iter()
        .find(|&&header| header.name == "Content-Length")
    {
        Some(header) => str::from_utf8(header.value).unwrap().parse().unwrap(),
        None => 0,
    };

    println!("BODY LENGTH {:?}", _body_length);

    // let request_body = read_request_body();

    match req.path {
        Some(path) => {
            if path.starts_with("/files") {
                serve_static_file(stream, &path[7..]);
            } else if path == "/api/v1" {
                respond_hello_world(stream);
            /*} else if path.starts_with("/cgi") {
                // DANGER CODE - JUST FOR TESTING
                handle_cgi_script(req, stream, client_addr, &path[5..]);*/
            } else {
                respond_file_not_found(stream);
            }
        }
        None => {
            respond_error(stream);
        }
    };
}

/**
 * main
 */
fn main() {
    let mut servers = vec![];

    unsafe {
        MIME_TYPES = Some(conduit_mime_types::Types::new().unwrap());
    }

    /*
     * http part - should force redirect to https by design
     */
    servers.push(thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

        loop {
            match listener.accept() {
                Ok((stream, remote_addr)) => thread::spawn(move || {
                    handle_http_scheme(stream, remote_addr);
                }),
                Err(e) => thread::spawn(move || println!("Connection failed: {:?}", e)),
            };
        }
    }));

    servers.push(thread::spawn(move || {
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
            match stream {
                Ok(stream) => {
                    let remote_addr = stream.peer_addr().unwrap();
                    let acceptor = acceptor.clone();
                    thread::spawn(move || {
                        match acceptor.accept(stream) {
                            Ok(stream) => thread::spawn(move || {
                                println!("ACCEPT HTTPS. REMOTE {:?}", remote_addr);
                                handle_request(stream, remote_addr);
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
