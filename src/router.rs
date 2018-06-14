use super::{Handler, Matcher, Method, ReadWrite, Route};

extern crate conduit_mime_types;
extern crate httparse;
extern crate openssl;

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::str;
use std::str::FromStr;

#[allow(dead_code)]
pub struct Router {
    options_routes: Vec<Route>,
    get_routes: Vec<Route>,
    post_routes: Vec<Route>,
    put_routes: Vec<Route>,
    delete_routes: Vec<Route>,
    head_routes: Vec<Route>,
    trace_routes: Vec<Route>,
    connect_routes: Vec<Route>,
    patch_routes: Vec<Route>,
    mime_types: conduit_mime_types::Types,
}

impl Router {
    pub fn new() -> Router {
        let _mime_types = conduit_mime_types::Types::new().unwrap();

        Router {
            options_routes: vec![],
            get_routes: vec![],
            post_routes: vec![],
            put_routes: vec![],
            delete_routes: vec![],
            head_routes: vec![],
            trace_routes: vec![],
            connect_routes: vec![],
            patch_routes: vec![],
            mime_types: _mime_types,
        }
    }

    fn respond_hello_world<W: Write>(&self, mut stream: W) {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
        stream.write(response).expect("Write failed");
    }

    fn serve_static_file<W: Write>(&self, mut stream: W, path: &str) {
        println!("STATIC {:?}", format!("www/{}", path));
        let mime_type = self.mime_types.mime_for_path(Path::new(path));
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

    fn respond_error<W: Write>(&self, mut stream: W) {
        let response = b"HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>500 - Server Error</body></html>\r\n";
        stream.write(response).expect("Write failed");
    }

    fn respond_file_not_found<W: Write>(&self, mut stream: W) {
        let response = b"HTTP/1.1 404 File Not Found\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>404 - File Not Found</body></html>\r\n";
        stream.write(response).expect("Write failed");
    }

    pub fn options(mut self, _matcher: Box<Matcher>, handler: Handler) -> Router {
        // path: &str
        // let mut regex = "^".to_string();
        // regex.push_str(path);
        // regex.push_str("$");
        // Path { matcher: Regex::new(&regex).unwrap() }
        self.options_routes.push(Route::new(/*matcher,*/ handler));
        self
    }

    fn read_request_head<T: Read>(&self, stream: T) -> Vec<u8> {
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

    // #[allow(dead_code)]
    pub fn to_https_scheme<T: ReadWrite>(&self, mut stream: T, _client_addr: SocketAddr) {
        let request_bytes = self.read_request_head(Read::by_ref(&mut stream));
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

    pub fn check(&self) {
        println!("ROUTER CHECK");
    }

    pub fn handle<T: ReadWrite>(&self, mut stream: T, _client_addr: SocketAddr) {
        let request_bytes = self.read_request_head(Read::by_ref(&mut stream));
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = httparse::Request::new(&mut headers);
        req.parse(&request_bytes).unwrap();

        match req.path {
            Some(ref path) => {
                println!("HTTP VERSION HTTP 1.{:?}", req.version.unwrap());

                let host = match req.headers.iter().find(|&&header| header.name == "Host") {
                    Some(header) => str::from_utf8(header.value).unwrap(),
                    None => "",
                };

                let method = Method::from_str(req.method.unwrap());

                println!("HOST {:?} {:?} {}\n", host, method.unwrap(), path);

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
                            self.serve_static_file(stream, &path[7..]);
                        } else if path == "/api/v1" {
                            self.respond_hello_world(stream);
                        /*} else if path.starts_with("/cgi") {
                    // DANGER CODE - JUST FOR TESTING
                    handle_cgi_script(req, stream, client_addr, &path[5..]);*/
                        } else {
                            self.respond_file_not_found(stream);
                        }
                    }
                    None => {
                        self.respond_error(stream);
                    }
                };
            }
            None => {
                println!("ROUTER ERROR");
            }
        }
    }
}
