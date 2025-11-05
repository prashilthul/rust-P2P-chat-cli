use tokio::net::UdpSocket;
use std::net::SocketAddr;

const DISCOVERY_MSG: &str = "p2p-chat-discovery";

/// Continuously broadcasts a UDP message to the local network to announce the peer's
/// presence. The message includes a discovery string and the port the peer is
/// listening on. This allows other peers to discover and connect to this peer.
pub async fn broadcast_presence(listen_port: u16) -> anyhow::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;
    let broadcast_addr = "255.255.255.255:8888".parse::<SocketAddr>()?;

    let msg = format!("{}:{}", DISCOVERY_MSG, listen_port);

    loop {
        socket.send_to(msg.as_bytes(), &broadcast_addr).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

/// Listens for UDP broadcast messages from other peers. When a valid discovery
/// message is received, it extracts the peer's address and port and returns it.
/// This function is used by the `discover` command to find peers on the network.
pub async fn listen_for_peers() -> anyhow::Result<SocketAddr> {
    let socket = UdpSocket::bind("0.0.0.0:8888").await?;
    let mut buf = [0; 1024];

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let msg = String::from_utf8_lossy(&buf[..len]);

        if let Some(port_str) = msg.strip_prefix(&format!("{}:", DISCOVERY_MSG)) {
            if let Ok(port) = port_str.parse::<u16>() {
                let mut peer_addr = addr;
                peer_addr.set_port(port);
                return Ok(peer_addr);
            }
        }
    }
}
