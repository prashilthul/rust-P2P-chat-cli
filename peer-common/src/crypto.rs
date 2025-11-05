use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305,
};
use rand::{Rng, rngs::OsRng};
use x25519_dalek::{EphemeralSecret, PublicKey};
use sha2::{Sha256, Digest};
use base64::{engine::general_purpose, Engine as _};

/// Generates a new ephemeral keypair for the X25519 elliptic curve Diffie-Hellman
/// key exchange. This function is used to create a new set of keys for each chat
/// session, ensuring forward secrecy.
pub fn generate_keypair() -> (EphemeralSecret, PublicKey) {
    let secret = EphemeralSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    (secret, public)
}

/// Derives a shared secret from the local private key and the peer's public key.
/// This shared secret is then hashed using SHA-256 to create a 32-byte session key.
/// The session key is used for symmetric encryption of the chat messages.
pub fn derive_shared_key(secret: EphemeralSecret, peer_pub: &PublicKey) -> [u8; 32] {
    let shared_secret = secret.diffie_hellman(peer_pub);
    let hash = Sha256::digest(shared_secret.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

/// Encrypts a message using the XChaCha20-Poly1305 AEAD (Authenticated Encryption
/// with Associated Data) algorithm. This function takes a 32-byte session key and
/// a plaintext message, and returns the ciphertext and a 24-byte nonce.
pub fn encrypt_message(key: &[u8; 32], plaintext: &[u8]) -> (Vec<u8>, [u8; 24]) {
    let cipher = XChaCha20Poly1305::new(key.into());

    let mut rng = rand::thread_rng();
    let mut nonce_bytes = [0u8; 24];
    rng.fill(&mut nonce_bytes);
    let nonce = &nonce_bytes.into();

    let ciphertext = cipher.encrypt(nonce, plaintext)
        .expect("encryption failure!");

    (ciphertext, nonce_bytes)
}

/// Decrypts a message using the XChaCha20-Poly1305 AEAD algorithm. This function
/// takes a 32-byte session key, the ciphertext, and the 24-byte nonce that was
/// used to encrypt the message. It returns the original plaintext message.
pub fn decrypt_message(key: &[u8; 32], ciphertext: &[u8], nonce_bytes: &[u8; 24]) -> Vec<u8> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce = (*nonce_bytes).into();

    cipher.decrypt(&nonce, ciphertext)
        .expect("decryption failure!")
}

/// Encodes a public key into a base64 string. This is used to transmit the public
/// key over the network in a safe and portable way.
pub fn pubkey_to_b64(pubkey: &PublicKey) -> String {
    general_purpose::STANDARD.encode(pubkey.as_bytes())
}

/// Decodes a base64 string into a public key. This is used to receive a public
/// key from a peer over the network.
pub fn pubkey_from_b64(b64: &str) -> anyhow::Result<PublicKey> {
    let bytes = general_purpose::STANDARD.decode(b64)?;
    let array: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("Invalid public key length"))?;
    Ok(PublicKey::from(array))
}

