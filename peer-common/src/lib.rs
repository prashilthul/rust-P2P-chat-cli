pub mod crypto;
pub mod types;
pub use types::WireMessage;
pub use x25519_dalek::{EphemeralSecret, PublicKey};

/// Represents the different types of symmetric encryption algorithms that can be used
/// in a session. This allows for flexibility in the choice of encryption algorithm.
#[derive(Clone, Debug)]
pub enum CipherType {
    AES256GCM,
    XChaCha20Poly1305,
}

/// Represents a secure chat session between two peers. It holds the 32-byte session
/// key and the selected cipher for encrypting and decrypting messages.
#[derive(Clone)]
pub struct Session {
    pub key: [u8; 32],
    pub cipher: CipherType,
}

impl Session {
    /// Creates a new `Session` from a 32-byte shared key. The default cipher used is
    /// XChaCha20Poly1305.
    pub fn new(key: [u8; 32]) -> Self {
        Session {
            key,
            cipher: CipherType::XChaCha20Poly1305,
        }
    }

    /// Encrypts a plaintext message using the selected cipher for the session. This
    /// method returns the ciphertext and the nonce used for encryption.
    pub fn encrypt(&self, plaintext: &[u8]) -> (Vec<u8>, Vec<u8>) {
        match self.cipher {
            CipherType::AES256GCM => {
                panic!("AES256GCM not yet implemented");
            }
            CipherType::XChaCha20Poly1305 => {
                let (ct, nonce) = crypto::encrypt_message(&self.key, plaintext);
                (ct, nonce.to_vec())
            }
        }
    }

    /// Decrypts a ciphertext message using the selected cipher for the session. This
    /// method takes the ciphertext and the nonce, and returns the original plaintext.
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8]) -> Vec<u8> {
        match self.cipher {
            CipherType::AES256GCM => {
                panic!("AES256GCM not yet implemented");
            }
            CipherType::XChaCha20Poly1305 => {
                let mut nonce_array = [0u8; 24];
                nonce_array.copy_from_slice(&nonce[..24]);
                crypto::decrypt_message(&self.key, ciphertext, &nonce_array)
            }
        }
    }
}
