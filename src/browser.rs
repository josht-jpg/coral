use std::{
    collections::HashMap,
    env,
    io::{self, BufRead, Read, Write},
    sync::Arc,
};

use anyhow::Result;
use fltk::{enums::*, prelude::*, window::Window, *};

pub struct Browser {
    window: Window,
}

impl Browser {
    pub fn new() -> Self {
        let args: Vec<String> = env::args().collect();
        let url = parse_url(&args[1]).unwrap();
        let (_headers, body) = request(&url);

        let mut buf = text::TextBuffer::default();
        buf.set_text(&lex(&body));

        let app = app::App::default();
        let mut window = window::Window::new(100, 100, 800, 600, "My Window");
        let mut txt = text::TextDisplay::new(0, 0, 800, 600, "");
        txt.set_buffer(buf);

        window.end();
        window.show();

        app.run().unwrap();

        Self { window }
    }
}

fn parse_url(url: &str) -> Result<Url> {
    // TODO: check that url starts with http:// or https://

    let (scheme, url) = url.split_once("://").unwrap();
    let first_slash_index = url.find('/').unwrap_or(url.len());
    let host = &url[..first_slash_index];
    let path = &url[first_slash_index..];

    let (host, port) = if let Some((host, port)) = host.split_once(':') {
        let port = port.parse::<u16>().ok();
        (host, port)
    } else {
        (host, None)
    };

    Ok(Url {
        scheme: scheme.try_into().unwrap(),
        port,
        host: host.to_owned(),
        path: path.to_owned(),
    })
}

fn lex(body: &str) -> String {
    let mut parsed_body = String::new();
    let mut in_angle = false;
    for c in body.chars() {
        if c == '<' {
            in_angle = true;
        } else if c == '>' {
            in_angle = false;
        } else if !in_angle {
            parsed_body.push(c);
        }
    }
    parsed_body
}

fn parse_response(response: &[u8]) -> (Headers, String) {
    let response = String::from_utf8(response.to_vec()).unwrap();

    let mut response_lines = response.split("\r\n");
    let status_line = response_lines
        .next()
        .unwrap()
        .split(' ')
        .collect::<Vec<&str>>();
    let (version, status, reason): (&str, &str, &str) =
        (status_line[0], status_line[1], status_line[2]);

    let mut headers = HashMap::new();
    loop {
        let Some(line) = &response_lines.next() else {
            break;
        };
        if line.trim().is_empty() {
            break;
        }

        let line = line.split(":").collect::<Vec<&str>>();
        let (header, value) = (line[0], line[1]);
        headers.insert(header.to_lowercase(), value.trim().to_owned());
    }

    (headers, response_lines.collect::<String>())
}

// TODO: lol big time duplication
fn https_request(url: &Url) -> (Headers, String) {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let rc_config = Arc::new(config);
    let mut client =
        rustls::ClientConnection::new(rc_config, url.host.as_str().try_into().unwrap()).unwrap();

    let mut socket = std::net::TcpStream::connect((url.host.clone(), 443)).unwrap();

    // TODO: should check if client wants write and socket is ready for write
    // client.write_tls(&mut socket);

    client
        .writer()
        .write(format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", url.path, url.host).as_bytes())
        .unwrap();

    let mut response = Vec::new();
    loop {
        if client.wants_read()
        /*&& socket.ready_for_read()*/
        {
            client.read_tls(&mut socket).unwrap();
            client.process_new_packets().unwrap();

            let mut plaintext = Vec::new();
            // TODO: clean up
            if let Err(_) = client.reader().read_to_end(&mut plaintext) {
                if client.wants_write() {
                    client.write_tls(&mut socket).unwrap();
                }
                continue;
            }
            response.extend(plaintext);
            if response.ends_with(b"</html>\n") {
                break;
            }
        }

        if client.wants_write() {
            client.write_tls(&mut socket).unwrap();
        }
    }

    // println!("{:?}", String::from_utf8(response.clone()));

    parse_response(&response)
}

fn http_request(url: &Url) -> (Headers, String) {
    let mut stream = std::net::TcpStream::connect((url.host.clone(), 80)).unwrap();
    stream
        .write(format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", url.path, url.host).as_bytes())
        .unwrap();

    let mut reader = io::BufReader::new(&mut stream);
    let mut response: Vec<u8> = Vec::new();
    loop {
        let received: Vec<u8> = reader.fill_buf().unwrap().to_vec();
        reader.consume(received.len());
        response.extend(received);
        if response.ends_with(b"</html>\n") {
            break;
        }
    }

    parse_response(&response)
}

fn request(url: &Url) -> (Headers, String) {
    match url.scheme {
        Scheme::Http => http_request(&url),
        Scheme::Https => https_request(&url),
    }
}

#[derive(Debug)]
enum Scheme {
    Http,
    Https,
}

impl TryFrom<&str> for Scheme {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "http" => Ok(Scheme::Http),
            "https" => Ok(Scheme::Https),
            _ => Err(anyhow::anyhow!("Unknown scheme")),
        }
    }
}

#[derive(Debug)]
struct Url {
    scheme: Scheme,
    host: String,
    port: Option<u16>,
    path: String,
}

type Headers = HashMap<String, String>;
