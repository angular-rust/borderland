extern crate httparse;
extern crate openssl;

use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod, SslStream};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str;
use std::sync::Arc;
use std::thread;

fn respond_hello_world(mut stream: TcpStream) {
    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
    stream.write(response).expect("Write failed");
}

fn serve_static_file(mut stream: TcpStream, path: &str) {
    println!("{:?}", format!("www/{}", path));
    let mut file = match File::open(format!("www/{}", path)) {
        Ok(file) => file,
        Err(_) => File::open("404.html").expect("404.html file missing!"),
    };

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Read failed");

    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n";
    stream.write(response).expect("Write failed");
    stream.write(&buffer).expect("Write failed");
}

fn respond_error(mut stream: TcpStream) {
    let response = b"HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>500 - Server Error</body></html>\r\n";
    stream.write(response).expect("Write failed");
}

fn respond_file_not_found(mut stream: TcpStream) {
    let response = b"HTTP/1.1 404 File Not Found\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>404 - File Not Found</body></html>\r\n";
    stream.write(response).expect("Write failed");
}

fn read_request_head(stream: &TcpStream) -> Vec<u8> {
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

/**
 * routing
 */
fn handle_request(stream: TcpStream, _client_addr: SocketAddr) {
    let request_bytes = read_request_head(&stream);
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);
    let _parsed = req.parse(&request_bytes);

    println!("VERSION {:?}", req.version.unwrap());
    println!("METHOD {:?}", req.method.unwrap());
    println!("PATH {:?}\n", req.path.unwrap());

    for (_i, elem) in req.headers.iter_mut().enumerate() {
        let s = match str::from_utf8(elem.value) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };

        println!("{:?} {:?}", elem.name, s)
    }

    // HTTP/1.1 301 Moved Permanently
    // Location: http://www.example.org/index.asp

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

    servers.push(thread::spawn(move || {
        println!("MOVED HTTP");
        let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

        loop {
            match listener.accept() {
                Ok((stream, addr)) => thread::spawn(move || {
                    handle_request(stream, addr);
                }),
                Err(e) => thread::spawn(move || println!("Connection failed: {:?}", e)),
            };
        }
    }));

    servers.push(thread::spawn(move || {
        println!("MOVED HTTPS");
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

        fn handle_client(mut stream: SslStream<TcpStream>) {
            println!("ACCEPT");
            let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
            stream.write(response).expect("Write failed");
        }

        for stream in listener.incoming() {
            println!("INCOMING");
            match stream {
                Ok(stream) => {
                    let acceptor = acceptor.clone();
                    thread::spawn(move || {
                        let stream = acceptor.accept(stream).unwrap();
                        handle_client(stream);
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
