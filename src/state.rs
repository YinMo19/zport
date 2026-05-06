use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub fn zport_dir() -> PathBuf {
    home::home_dir().unwrap_or_default().join(".zport")
}
pub fn state_path() -> PathBuf {
    zport_dir().join("state.json")
}
pub fn sock_path(host: &str) -> PathBuf {
    zport_dir().join(format!("mux-{host}"))
}

#[derive(Serialize, Deserialize, Default)]
pub struct State {
    pub default_host: Option<String>,
    #[serde(default)]
    pub hosts: HashMap<String, HashMap<String, Entry>>,
    #[serde(default)]
    pub ports: HashMap<String, u16>,
}

// `untagged` lets serde distinguish Cm vs Proc by the presence of `pid`.
#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Entry {
    Cm { remote: u16 },
    Proc { remote: u16, pid: u32 },
}

impl Entry {
    pub fn remote(&self) -> u16 {
        match self {
            Entry::Cm { remote } | Entry::Proc { remote, .. } => *remote,
        }
    }
    pub fn pid(&self) -> Option<u32> {
        match self {
            Entry::Proc { pid, .. } => Some(*pid),
            _ => None,
        }
    }
}

pub fn load() -> State {
    fs::read_to_string(state_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(state: &State) {
    let _ = fs::create_dir_all(zport_dir());
    let _ = fs::write(
        state_path(),
        serde_json::to_string_pretty(state).unwrap() + "\n",
    );
}
