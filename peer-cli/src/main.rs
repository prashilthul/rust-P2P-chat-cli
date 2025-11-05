use peer_core::{start_listener, start_client, listen_for_peers, persistence::Persist};
use std::env;
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();

    // If no command is provided, print usage information and exit
    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  {} listen <ADDR:PORT>", args[0]);
        eprintln!("  {} connect <ALIAS|ADDR:PORT>", args[0]);
        eprintln!("  {} discover", args[0]);
        eprintln!("  {} add-peer <ALIAS> <ADDR:PORT>", args[0]);
        eprintln!("  {} list-peers", args[0]);
        return Ok(());
    }

    // Load the persisted peer data
    let mut persist = Persist::load();

    // Dispatch the command to the appropriate handler
    match args[1].as_str() {
        "listen" => {
            if args.len() != 3 {
                eprintln!("Usage: {} listen <ADDR:PORT>", args[0]);
                return Ok(());
            }
            // Start the listener
            start_listener(&args[2]).await?;
        }

        "connect" => {
            if args.len() != 3 {
                eprintln!("Usage: {} connect <ALIAS|ADDR:PORT>", args[0]);
                return Ok(());
            }
            // If the provided address is an alias, get the corresponding address from the
            // persisted data. Otherwise, use the provided address directly.
            let addr = persist.get_peer(&args[2]).map(|p| p.addr.clone()).unwrap_or(args[2].clone());
            // Start the client and connect to the peer
            start_client(&addr).await?;
        }

        "discover" => {
            use std::collections::HashSet;
            use std::io::{stdin, stdout, Write};

            let mut discovered_peers = HashSet::new();
            println!("Searching for peers... (Press Ctrl+C to stop)");

            // Loop to discover peers on the network
            loop {
                match tokio::time::timeout(std::time::Duration::from_secs(5), listen_for_peers()).await {
                    Ok(Ok(peer_addr)) => {
                        if discovered_peers.insert(peer_addr) {
                            println!("Found peer: {}", peer_addr);
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("Discovery error: {}", e);
                    }
                    Err(_) => {
                        // If no peers have been discovered yet, continue searching. Otherwise,
                        // break the loop and present the list of discovered peers to the user.
                        if discovered_peers.is_empty() {
                            println!("No peers found yet...");
                        } else {
                            break;
                        }
                    }
                }
            }

            if discovered_peers.is_empty() {
                println!("No peers found.");
                return Ok(());
            }

            // Present the list of discovered peers to the user
            let peers: Vec<_> = discovered_peers.into_iter().collect();
            println!("\nDiscovered peers:");
            for (i, peer) in peers.iter().enumerate() {
                println!("  [{}] {}", i, peer);
            }

            // Prompt the user to select a peer to connect to
            print!("\nEnter the number of the peer to connect to (or 'q' to quit): ");
            stdout().flush()?;

            let mut choice = String::new();
            stdin().read_line(&mut choice)?;

            if choice.trim() == "q" {
                return Ok(());
            }

            // Parse the user's choice and connect to the selected peer
            if let Ok(n) = choice.trim().parse::<usize>() {
                if n < peers.len() {
                    let peer_addr = peers[n];
                    // Prompt the user for an optional alias for the peer
                    print!("Enter an alias for this peer (optional): ");
                    stdout().flush()?;
                    let mut alias = String::new();
                    stdin().read_line(&mut alias)?;
                    let alias = alias.trim();

                    // If an alias is provided, save the peer to the persisted data
                    if !alias.is_empty() {
                        persist.add_peer(alias.to_string(), peer_addr.to_string());
                        persist.save()?;
                        println!("Peer '{}' saved.", alias);
                    }

                    // Connect to the selected peer
                    println!("Connecting to {}...", peer_addr);
                    start_client(&peer_addr.to_string()).await?;
                } else {
                    eprintln!("Invalid selection.");
                }
            } else {
                eprintln!("Invalid input.");
            }
        }

        "add-peer" => {
            if args.len() != 4 {
                eprintln!("Usage: {} add-peer <ALIAS> <ADDR:PORT>", args[0]);
                return Ok(());
            }
            // Add the peer to the persisted data and save it to the configuration file
            persist.add_peer(args[2].clone(), args[3].clone());
            persist.save()?;
            println!("Peer '{}' added.", args[2]);
        }

        "list-peers" => {
            // List all the saved peers
            println!("Saved peers:");
            for peer in persist.list_peers() {
                println!("  - {}: {}", peer.name, peer.addr);
            }
        }

        _ => {
            eprintln!("Unknown command: {}", args[1]);
        }
    }

    Ok(())
}
