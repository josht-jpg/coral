use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use telnet::{Event, Telnet};

fn parse_url(url: &str) -> Result<Url> {
    if !url.starts_with("http://") {
        return Err(anyhow!("URL must start with http://"));
    }
    let url = &url["http://".len()..];
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
    let example_com = "example.com".try_into().unwrap();
    let mut client = rustls::ClientConnection::new(rc_config, example_com);

    let mut stream = std::net::TcpStream::connect((url.host.clone(), 443)).unwrap();
    let mut tls_stream = rustls::Stream::new(&mut client, &mut stream);

    // tls_stream
    //     .write(format!("GET {} HTTP/1.0\r
}

fn request(url: &Url) -> (Headers, String) {
    let scheme = url.split("://").collect::<Vec<&str>>()[0];
    if scheme == "https" {
        return https_request(&url);
    }

    let mut telnet = Telnet::connect((url.host.clone(), 80), 256).unwrap();

    telnet.write(format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", url.path, url.host).as_bytes());

    let mut buffer: Vec<u8> = Vec::new();
    loop {
        let event = &telnet.read().expect("Read error");

        if let Event::Data(b) = event {
            buffer.extend(b.iter());
            if buffer.ends_with(b"</html>\n") {
                break;
            }
        }
    }

    let response = String::from_utf8(buffer.to_vec()).unwrap();

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
    let url = parse_url("http://example.org/index.html").unwrap();
    let (headers, body) = request(&url);
    show_body(&body);

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
    let example_com = "example.com".try_into().unwrap();
    let mut client = rustls::ClientConnection::new(rc_config, example_com);
}

struct Url {
    host: String,
    path: String,
}

type Headers = HashMap<String, String>;
