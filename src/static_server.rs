use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};

pub fn serve(root: PathBuf, addr: SocketAddr) {
    let listener = TcpListener::bind(addr).expect("failed to bind static file server");
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let root = root.clone();
        std::thread::spawn(move || handle(&mut stream, &root));
    }
}

fn handle(stream: &mut std::net::TcpStream, root: &Path) {
    let mut buf = [0u8; 4096];
    let Ok(n) = stream.read(&mut buf) else { return };
    let req = std::str::from_utf8(&buf[..n]).unwrap_or("");

    let raw_path = req.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    // Strip query string and fragment
    let raw_path = raw_path.split(&['?', '#'][..]).next().unwrap_or("/");
    let raw_path = if raw_path == "/" { "/index.html" } else { raw_path };

    let rel = raw_path.trim_start_matches('/');
    if rel.contains("..") {
        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n");
        return;
    }

    let file_path = root.join(rel);
    match std::fs::read(&file_path) {
        Ok(data) => {
            let ct = content_type(rel);
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {ct}\r\nContent-Length: {}\r\n\r\n",
                data.len()
            );
            let _ = stream.write_all(&data);
        }
        Err(_) => {
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
        }
    }
}

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".html")      { "text/html; charset=utf-8" }
    else if path.ends_with(".css")  { "text/css" }
    else if path.ends_with(".js")   { "application/javascript" }
    else if path.ends_with(".svg")  { "image/svg+xml" }
    else                            { "application/octet-stream" }
}
