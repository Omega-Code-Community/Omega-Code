use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use log::{info, warn, error};

pub fn check_network() -> bool {
    let targets = [
        "1.1.1.1:443",      // Cloudflare
        "8.8.8.8:53",       // Google DNS
        "223.5.5.5:53",     // AliDNS
        "180.76.76.76:53",  // Baidu DNS
    ];

    info!("Starting network connectivity check");

    for target in targets {
        if can_connect(target, Duration::from_secs(3)) {
            info!("Network check passed, successfully connected to {}", target);
            return true;
        } else {
            warn!("Failed to connect to {}", target);
        }
    }

    error!("All network targets unreachable, network check failed");
    false
}

fn can_connect(addr: &str, timeout: Duration) -> bool {
    let addrs = match addr.to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(e) => {
            warn!("Failed to resolve address {}: {}", addr, e);
            return false;
        }
    };

    for addr in addrs {
        if TcpStream::connect_timeout(&addr, timeout).is_ok() {
            return true;
        }
    }

    false
}