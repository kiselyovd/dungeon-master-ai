//! Bind 127.0.0.1:0, read assigned port, drop listener, hand port to caller.
//! Race window is ~50ms - acceptable for desktop solo-user environment.

use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

pub fn discover_free_port() -> io::Result<u16> {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_returns_unique_free_ports() {
        let p1 = discover_free_port().unwrap();
        let p2 = discover_free_port().unwrap();
        assert_ne!(p1, p2);
        assert!(p1 > 1024);
        assert!(p2 > 1024);
    }
}
