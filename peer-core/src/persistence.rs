use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::io::Write;

/// Represents the configuration for a single peer, including their name (alias),
/// address, and an optional public key. The public key is not currently used but
/// is reserved for future functionality.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerConfig {
    pub name: String,
    pub addr: String,
    pub pubkey_b64: Option<String>,
}

/// The main container for the application's persistent data, which is a list of
/// `PeerConfig`s. This struct is serialized to and from the configuration file.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Persist {
    pub peers: Vec<PeerConfig>,
}

/// Returns the path to the configuration file, which is `.p2p-chat.json` in the
/// user's home directory.
fn get_config_path() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(".p2p-chat.json"))
}

impl Persist {
    /// Loads the persisted data from the configuration file. If the file doesn't exist
    /// or is invalid, it returns a default `Persist` instance.
    pub fn load() -> Self {
        if let Ok(path) = get_config_path() {
            if let Ok(s) = fs::read_to_string(path) {
                if let Ok(p) = serde_json::from_str(&s) {
                    return p;
                }
            }
        }
        Persist::default()
    }

    /// Saves the current state of the `Persist` struct to the configuration file in a
    /// pretty-printed JSON format.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = get_config_path()?;
        let mut f = fs::File::create(path)?;
        f.write_all(serde_json::to_string_pretty(self)?.as_bytes())?;
        Ok(())
    }

    /// Adds a new peer to the list of peers. If a peer with the same name already
    /// exists, it is replaced.
    pub fn add_peer(&mut self, name: String, addr: String) {
        self.peers.retain(|p| p.name != name);
        self.peers.push(PeerConfig { name, addr, pubkey_b64: None });
    }

    /// Retrieves a peer by their name (alias).
    pub fn get_peer(&self, name: &str) -> Option<&PeerConfig> {
        self.peers.iter().find(|p| p.name == name)
    }

    /// Returns a reference to the list of all saved peers.
    pub fn list_peers(&self) -> &Vec<PeerConfig> {
        &self.peers
    }
}
