use std::{
    collections::HashMap,
    io::{self, BufRead, Read, Write},
    sync::Arc,
};

use anyhow::{anyhow, Result};
use telnet::{Event, Telnet};

fn parse_url(url: &str) -> Result<Url> {
    // TODO: check that url starts with http:// or https://

    // TODO: clean up lol
    let url = &url[(if url.starts_with("http://") {
        "http://".len()
    } else {
        "https://".len()
    })..];
    let first_slash_index = url.find('/').unwrap_or(url.len());
    let host = &url[..first_slash_index];
    let path = &url[first_slash_index..];

    Ok(Url {
        host: host.to_owned(),
        path: path.to_owned(),
    })
}

fn show_body(body: &str) {
    let mut in_angle = false;
    for c in body.chars() {
        if c == '<' {
            in_angle = true;
        } else if c == '>' {
            in_angle = false;
        } else if !in_angle {
            print!("{}", c);
        }
    }
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
            client.reader().read_to_end(&mut plaintext).unwrap();
            // io::stdout().write(&plaintext).unwrap();
            response.extend(plaintext);
            if response.ends_with(b"</html>\n") {
                break;
            }
        }
    }

    let response = String::from_utf8(response).unwrap();

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
        let line = &response_lines.next().unwrap();
        if line.trim().is_empty() {
            break;
        }

        let line = line.split(":").collect::<Vec<&str>>();
        let (header, value) = (line[0], line[1]);
        headers.insert(header.to_lowercase(), value.trim().to_owned());
    }

    (headers, response_lines.collect::<String>())
}

fn request(url: &Url) -> (Headers, String) {
    let scheme = url.host.split("://").collect::<Vec<&str>>()[0];

    if scheme == "https" {
        return https_request(&url);
    }

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

    let response = String::from_utf8(response).unwrap();

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
        let line = &response_lines.next().unwrap();
        if line.trim().is_empty() {
            break;
        }

        let line = line.split(":").collect::<Vec<&str>>();
        let (header, value) = (line[0], line[1]);
        headers.insert(header.to_lowercase(), value.trim().to_owned());
    }

    (headers, response_lines.collect::<String>())
}

fn main() {
    let url = parse_url("https://example.org/index.html").unwrap();
    let (headers, body) = request(&url);
    show_body(&body);
}

struct Url {
    host: String,
    path: String,
}

type Headers = HashMap<String, String>;
