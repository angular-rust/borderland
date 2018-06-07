use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str;
use std::thread;

extern crate httparse;

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

    println!("{:?}", req.method.unwrap());
    println!("{:?}\n", req.path.unwrap());

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
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    loop {
        match listener.accept() {
            Ok((stream, addr)) => thread::spawn(move || {
                handle_request(stream, addr);
            }),
            Err(e) => thread::spawn(move || println!("Connection failed: {:?}", e)),
        };
    }
}
