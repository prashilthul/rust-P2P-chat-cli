use serde::{Deserialize, Serialize};

/// Represents all the possible messages that can be exchanged between peers. This enum
/// is the core data structure for all communication. Messages are serialized to JSON
/// and sent over the wire with a 4-byte big-endian length prefix.
#[derive(Serialize, Deserialize, Debug)]
pub enum WireMessage {
    /// Used to exchange public keys and establish a secure session. The `pubkey` field
    /// contains the base64-encoded public key of the sender.
    Handshake { pubkey: String },

    /// Used to send encrypted chat messages. The `payload` field contains the
    /// base64-encoded ciphertext of the message, and the `nonce` field contains the
    /// base64-encoded 24-byte nonce that was used to encrypt the message.
    Chat { sender_id: String, timestamp: u64, payload: String, nonce: String },

    /// Used to acknowledge the receipt of a message. The `id` field contains the ID
    /// of the message being acknowledged.
    Ack { id: String },

    /// Used to keep the connection alive and check if the peer is still responsive.
    Ping,
}
