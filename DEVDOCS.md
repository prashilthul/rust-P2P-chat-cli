# Developer Documentation

This document provides a technical overview of the P2P Encrypted Chat application for developers who want to understand the codebase.

## Project Structure

The project is a Rust workspace composed of three crates:

*   `peer-common`: A library crate that contains the common data structures, cryptographic functions, and session management logic used by the other crates.
*   `peer-core`: A library crate that implements the core functionality of the application, including networking, peer discovery, and persistence.
*   `peer-cli`: A binary crate that provides a command-line interface (CLI) for users to interact with the application.

This modular structure separates the core logic from the UI, making the code easier to maintain and test.

## Core Concepts

### Cryptography

The end-to-end encryption is implemented in the `peer-common/src/crypto.rs` file.

*   **Key Exchange**: The application uses the X25519 elliptic curve Diffie-Hellman (ECDH) key exchange to establish a shared secret between two peers. Each peer generates an ephemeral keypair and they exchange their public keys.
*   **Shared Secret**: The shared secret is derived using the `diffie_hellman` method. This secret is then hashed using SHA-256 to create a 32-byte session key.
*   **Encryption**: All messages are encrypted using the XChaCha20-Poly1305 AEAD (Authenticated Encryption with Associated Data) algorithm. This provides both confidentiality and integrity for the messages.

### Networking

The networking logic is implemented in the `peer-core/src/net.rs` file.

*   **Transport Protocol**: The application uses TCP for reliable, ordered communication between peers.
*   **Message Framing**: All messages are sent as length-prefixed JSON payloads. A 4-byte big-endian integer representing the length of the message is sent before the message itself. This allows the receiver to know how many bytes to read for each message.
*   **`WireMessage` Enum**: The `peer-common/src/types.rs` file defines the `WireMessage` enum, which represents all the possible messages that can be exchanged between peers. This includes messages for the handshake, chat messages, and acknowledgments.
*   **Handshake**: When two peers connect, they perform a handshake to establish a secure session. The client sends a `Handshake` message with its public key. The listener receives this message, derives the shared secret, and sends back its own `Handshake` message. Once both peers have derived the shared secret, the secure session is established.

### Peer Discovery

The peer discovery mechanism is implemented in the `peer-core/src/discovery.rs` file.

*   **UDP Broadcasts**: The application uses UDP broadcasts on the local network to discover other peers. A peer in "listen" mode will periodically send a broadcast message containing the string "p2p-chat-discovery" and the port it is listening on.
*   **Listening for Broadcasts**: A peer in "discover" mode will listen for these UDP broadcast messages. When a message is received, the peer extracts the sender's IP address and the port from the message and adds them to a list of discovered peers.

### Persistence

Peer data is persisted to a JSON file in the user's home directory. The logic for this is in the `peer-core/src/persistence.rs` file.

*   **`.p2p-chat.json`**: This file stores a list of saved peers, including their aliases and addresses.
*   **`Persist` Struct**: The `Persist` struct provides methods for loading, saving, adding, and retrieving peer information from the JSON file.

## Code Walkthrough

Here is a high-level walkthrough of the code execution flow:

1.  **`main.rs` in `peer-cli`**: The application starts in the `main` function of `peer-cli/src/main.rs`. It parses the command-line arguments to determine which command to execute.
2.  **Command Dispatch**: The `match` statement in `main` dispatches the command to the appropriate handler function.
3.  **`peer-core` Interaction**: The command handlers in `peer-cli` call functions from the `peer-core` crate to perform the actual work.
    *   `listen` -> `peer_core::start_listener()`
    *   `connect` -> `peer_core::start_client()`
    *   `discover` -> `peer_core::listen_for_peers()`
    *   `add-peer` -> `persist.add_peer()`
    *   `list-peers` -> `persist.list_peers()`
4.  **`start_listener()`**: This function in `peer-core/src/net.rs` binds to a TCP socket and starts listening for incoming connections. It also spawns a background task to broadcast the peer's presence using `discovery::broadcast_presence()`.
5.  **`start_client()`**: This function in `peer-core/src/net.rs` connects to a peer at a given address. It then initiates the handshake process.
6.  **`handle_conn()`**: This function in `peer-core/src/net.rs` is called for both the listener and the client once a connection is established. It handles the handshake and then enters the `chat_loop()`.
7.  **`chat_loop()`**: This function in `peer-core/src/net.rs` handles the interactive chat session. It spawns a task to read incoming messages from the socket and another loop to read user input from stdin.

## Crate Details

### `peer-common`

*   **`crypto.rs`**: Contains all the cryptographic functions for key generation, key derivation, encryption, and decryption.
*   **`types.rs`**: Defines the `WireMessage` enum, which is the core data structure for all communication between peers.
*   **`lib.rs`**: Defines the `Session` struct, which holds the session key and provides a high-level interface for encrypting and decrypting messages.

### `peer-core`

*   **`net.rs`**: Contains the core networking logic, including the TCP listener, client, handshake, and chat loop.
*   **`discovery.rs`**: Implements the UDP-based peer discovery mechanism.
*   **`persistence.rs`**: Handles the serialization and deserialization of peer data to and from the `.p2p-chat.json` file.

### `peer-cli`

*   **`main.rs`**: The entry point of the application. It parses command-line arguments and calls the appropriate functions in `peer-core`.
