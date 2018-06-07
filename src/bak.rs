use std::process::Command;

fn request_url(buffer: &[u8]) -> Option<&str> {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);

    match req.parse(&buffer) {
        Ok(_) => match req.path {
            Some(ref path) => {
                return Some(path);
            }
            None => {
                return None;
            }
        },
        Err(_) => {
            return None;
        }
    }
}

fn build_environmental_variables<'a>(
    command: &'a mut Command,
    meta_variables: Vec<(&'a str, &'a str)>,
) {
    for &tup in meta_variables.iter() {
        command.env(tup.0, tup.1);
    }

    println!("{:?}", command);
}

fn build_cgi_meta_vars<'a>(
    request: &'a httparse::Request,
    client_ip: &'a String,
    script_name: &'a str,
    path_info: &'a str,
) -> Vec<(&'a str, &'a str)> {
    let mut headers = Vec::new();

    for (_idx, &item) in request.headers.iter().enumerate() {
        match &item.name {
            &"Authorization" => headers.push(("AUTH_TYPE", str::from_utf8(&item.value).unwrap())),
            &"Content-Length" => {
                headers.push(("CONTENT_LENGTH", str::from_utf8(&item.value).unwrap()))
            }
            &"Content-Type" => headers.push(("CONTENT_TYPE", str::from_utf8(&item.value).unwrap())),
            &"Host" => {
                let header_value = str::from_utf8(&item.value).unwrap();

                match header_value.find(':') {
                    Some(index) => {
                        headers.push(("SERVER_NAME", &header_value[..(index)]));
                        headers.push(("SERVER_PORT", &header_value[(index + 1)..]));
                    }
                    None => {
                        headers.push(("SERVER_NAME", header_value));
                    }
                }
            }
            _ => {}
        };
    }

    headers.push(("REMOTE_ADDR", &client_ip[..]));
    headers.push(("REMOTE_HOST", &client_ip[..]));

    headers.push(("REQUEST_METHOD", request.method.unwrap()));
    headers.push(("SCRIPT_NAME", script_name));

    match path_info.find('?') {
        Some(index) => {
            headers.push(("PATH_INFO", &path_info[..(index)]));
            headers.push(("QUERY_STRING", &path_info[(index + 1)..]));
        }
        None => {
            headers.push(("PATH_INFO", path_info));
        }
    };

    headers.push(("SERVER_PROTOCOL", "HTTP 1.1"));
    headers.push(("SERVER_SOFTWARE", "rust-httpd 0.1"));

    return headers;
}

fn handle_cgi_script(
    request: httparse::Request,
    mut stream: TcpStream,
    client_addr: SocketAddr,
    req_path: &str,
) {
    let path_components: Vec<&str> = req_path.splitn(2, "/").collect();
    let default_path = "/";
    let (script_name, path_info) = (
        path_components.get(0).unwrap(),
        path_components.get(1).unwrap_or(&default_path),
    );

    let client_ip = client_addr.ip().to_string();

    let meta_variables = build_cgi_meta_vars(&request, &client_ip, script_name, path_info);

    let mut command = Command::new(format!("cgi/{}", script_name));

    println!("{:?}", &meta_variables);
    build_environmental_variables(&mut command, meta_variables);

    match command.output() {
        Ok(output) => {
            if output.status.success() {
                stream.write(&output.stdout).expect("Command failed");
            } else {
                stream.write(&output.stderr).expect("Stderr");
            }
        }
        Err(_) => {
            respond_error(stream);
        }
    }
}
