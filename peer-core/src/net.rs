use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use peer_common::types::WireMessage;
use peer_common::crypto::{generate_keypair, derive_shared_key, pubkey_to_b64, pubkey_from_b64};
use peer_common::Session;
use base64::{engine::general_purpose, Engine as _};

#[cfg(feature = "notify")]
use notify_rust::Notification;

/// Serializes a `WireMessage` to JSON, prefixes it with a 4-byte big-endian length,
/// and writes it to a `TcpStream`. This function is used to send messages to a peer.
async fn write_msg(stream: &mut TcpStream, wm: &WireMessage) -> anyhow::Result<()> {
    let v = serde_json::to_vec(wm)?;
    let len = (v.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&v).await?;
    Ok(())
}

/// Reads a length-prefixed JSON message from a `TcpStream` and deserializes it into
/// a `WireMessage`. This function is used to receive messages from a peer.
async fn read_msg(stream: &mut TcpStream) -> anyhow::Result<WireMessage> {
    let mut len_buf = [0u8;4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    let wm: WireMessage = serde_json::from_slice(&buf)?;
    Ok(wm)
}

/// Starts a TCP listener on the given address. For each incoming connection, it
/// spawns a new task to handle the connection. It also starts a background task to
/// broadcast the peer's presence on the network.
pub async fn start_listener(bind_addr: &str) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;
    let port = listener.local_addr()?.port();
    println!("Listening on {}", bind_addr);

    tokio::spawn(async move {
        if let Err(e) = crate::discovery::broadcast_presence(port).await {
            eprintln!("broadcast error: {:?}", e);
        }
    });

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        println!("Accepted connection from {}", peer_addr);
        tokio::spawn(async move {
            if let Err(e) = handle_conn(socket, true).await {
                eprintln!("connection error: {:?}", e);
            }
        });
    }
}

/// Connects to a peer at the given address and then calls `handle_conn` to handle
/// the connection.
pub async fn start_client(target: &str) -> anyhow::Result<()> {
    let stream = TcpStream::connect(target).await?;
    println!("Connected to {}", target);
    handle_conn(stream, false).await?;
    Ok(())
}

/// Handles the cryptographic handshake to establish a secure session, and then
/// enters the `chat_loop`. This function is called for both the listener and the
/// client.
async fn handle_conn(mut stream: TcpStream, is_listener: bool) -> anyhow::Result<()> {
    let (my_secret, my_pub) = generate_keypair();
    let my_pub_b64 = pubkey_to_b64(&my_pub);

    if is_listener {
        let incoming = read_msg(&mut stream).await?;
        match incoming {
            WireMessage::Handshake { pubkey } => {
                let peer_pub = pubkey_from_b64(&pubkey)?;
                let hm = WireMessage::Handshake { pubkey: my_pub_b64.clone() };
                write_msg(&mut stream, &hm).await?;
                let shared_key = derive_shared_key(my_secret, &peer_pub);
                let session = Session::new(shared_key);
                println!("🔐 Session key derived (listener)");
                chat_loop(stream, session, false).await?;
            }
            _ => {
                eprintln!("expected handshake");
            }
        }
    } else {
        let hm = WireMessage::Handshake { pubkey: my_pub_b64.clone() };
        write_msg(&mut stream, &hm).await?;
        let reply = read_msg(&mut stream).await?;
        match reply {
            WireMessage::Handshake { pubkey } => {
                let peer_pub = pubkey_from_b64(&pubkey)?;
                let shared_key = derive_shared_key(my_secret, &peer_pub);
                let session = Session::new(shared_key);
                println!("🔐 Session key derived (client)");
                chat_loop(stream, session, true).await?;
            }
            _ => eprintln!("expected handshake reply"),
        }
    }
    Ok(())
}

/// Handles the interactive chat session. It splits the `TcpStream` into a reader and
/// a writer, and then spawns a task to read incoming messages and another loop to
/// read user input from stdin.
async fn chat_loop(stream: TcpStream, session: Session, _pause_read: bool) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use std::io::{stdout, Write};
    use colored::Colorize;

    println!("🔒 Secure channel established. You can type messages now.");
    let (r, mut w) = stream.into_split();
    let mut reader = BufReader::new(r);
    let session = Arc::new(session);

    let session_rx = session.clone();
    let reader_task = tokio::spawn(async move {
        loop {
            match read_msg_from_reader(&mut reader, &session_rx).await {
                Ok(Some(text)) => {
                    let timestamp = chrono::Local::now().format("%H:%M:%S");
                    println!("\n{} {}: {}", timestamp.to_string().dimmed(), "Peer".yellow(), text);
                    #[cfg(feature = "notify")]
                    let _ = Notification::new().summary("New message").body(&text).show();
                    print!("> ");
                    let _ = std::io::stdout().flush();
                }
                Ok(None) => {
                    println!("\nPeer disconnected.");
                    break;
                }
                Err(e) => {
                    eprintln!("recv err: {:?}", e);
                    break;
                }
            }
        }
    });

    let mut stdin_reader = BufReader::new(tokio::io::stdin());
    let mut input = String::new();
    loop {
        input.clear();
        print!("> ");
        stdout().flush()?;
        let n = stdin_reader.read_line(&mut input).await?;
        if n == 0 { break; }
        let text = input.trim_end().to_string();
        if text.is_empty() { continue; }
        if text == "/quit" { break; }

        let (ct, nonce) = session.encrypt(text.as_bytes());
        let b64_ct = general_purpose::STANDARD.encode(&ct);
        let b64_nonce = general_purpose::STANDARD.encode(&nonce);

        let wm = WireMessage::Chat {
            sender_id: "me".to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            payload: b64_ct,
            nonce: b64_nonce,
        };
        write_msg_raw(&mut w, &wm).await?;
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        println!("{} {}: {}", timestamp.to_string().dimmed(), "You".green(), text);
    }

    let _ = reader_task.await;
    Ok(())
}

/// A helper function that reads a `WireMessage` from a reader that implements
/// `AsyncBufRead`, and decrypts chat messages.
async fn read_msg_from_reader<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R, session: &Session) -> anyhow::Result<Option<String>> {
    let mut lenb = [0u8;4];
    if let Err(e) = reader.read_exact(&mut lenb).await {
        return if e.kind() == std::io::ErrorKind::UnexpectedEof {
            Ok(None)
        } else {
            Err(e.into())
        };
    }
    let len = u32::from_be_bytes(lenb) as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    let wm: WireMessage = serde_json::from_slice(&buf)?;
    match wm {
        WireMessage::Chat { sender_id: _, timestamp: _, payload, nonce } => {
            let data = general_purpose::STANDARD.decode(&payload)?;
            let nonce_bytes = general_purpose::STANDARD.decode(&nonce)?;
            let pt = session.decrypt(&data, &nonce_bytes);
            let s = String::from_utf8_lossy(&pt).to_string();
            Ok(Some(s))
        }
        WireMessage::Ping => Ok(None),
        _ => Ok(None),
    }
}

/// A helper function that writes a `WireMessage` to a writer that implements
/// `AsyncWrite`.
async fn write_msg_raw<W: tokio::io::AsyncWrite + Unpin>(writer: &mut W, wm: &WireMessage) -> anyhow::Result<()> {
    let v = serde_json::to_vec(wm)?;
    let len = (v.len() as u32).to_be_bytes();
    writer.write_all(&len).await?;
    writer.write_all(&v).await?;
    Ok(())
}
