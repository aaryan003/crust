// Configuration module - handles ~/.crust/credentials and ~/.crust/config

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub server: String,
    pub username: String,
    pub token: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialsFile {
    pub credentials: Vec<Credentials>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub remotes: Vec<Remote>,
}

pub struct Config {
    remotes: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            remotes: HashMap::new(),
        }
    }

    pub fn load() -> Result<Self> {
        // Remotes are repo-local: read from .crust/config in current dir
        let local_config = std::path::Path::new(".crust/config");
        let mut config = Config::new();

        if local_config.exists() {
            let contents = fs::read_to_string(local_config)?;
            if !contents.trim().is_empty() {
                let config_data: ConfigFile =
                    serde_json::from_str(&contents).unwrap_or_else(|_| ConfigFile { remotes: vec![] });
                for remote in config_data.remotes {
                    config.remotes.insert(remote.name, remote.url);
                }
            }
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        // Remotes are repo-local: save to .crust/config in current dir
        let local_config = std::path::Path::new(".crust/config");
        let remotes: Vec<Remote> = self
            .remotes
            .iter()
            .map(|(name, url)| Remote {
                name: name.clone(),
                url: url.clone(),
            })
            .collect();

        let config_data = ConfigFile { remotes };
        let json = serde_json::to_string_pretty(&config_data)?;
        fs::write(local_config, json)?;
        Ok(())
    }

    pub fn add_remote(&mut self, name: String, url: String) -> Result<()> {
        if self.remotes.contains_key(&name) {
            return Err(anyhow!(
                "Remote '{}' already exists. Use 'crust remote remove {}' to remove it first.",
                name,
                name
            ));
        }
        self.remotes.insert(name, url);
        Ok(())
    }

    pub fn get_remote(&self, name: &str) -> Option<String> {
        self.remotes.get(name).cloned()
    }

    pub fn get_remotes(&self) -> Vec<(String, String)> {
        let mut remotes: Vec<_> = self
            .remotes
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        remotes.sort_by(|a, b| a.0.cmp(&b.0));
        remotes
    }

    pub fn delete_remote(&mut self, name: &str) -> Result<()> {
        self.remotes.remove(name);
        Ok(())
    }

    pub fn rename_remote(&mut self, old_name: &str, new_name: &str) -> Result<()> {
        if !self.remotes.contains_key(old_name) {
            return Err(anyhow!("No such remote: '{}'", old_name));
        }
        if self.remotes.contains_key(new_name) {
            return Err(anyhow!("Remote '{}' already exists", new_name));
        }
        let url = self.remotes.remove(old_name).unwrap();
        self.remotes.insert(new_name.to_string(), url);
        Ok(())
    }

    pub fn set_remote_url(&mut self, name: &str, url: &str) -> Result<()> {
        if !self.remotes.contains_key(name) {
            return Err(anyhow!("No such remote: '{}'", name));
        }
        self.remotes.insert(name.to_string(), url.to_string());
        Ok(())
    }
}

pub fn get_config_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not find home directory"))
        .map(|h| h.join(".crust"))
}

pub fn get_credentials_file() -> Result<PathBuf> {
    get_config_dir().map(|d| d.join("credentials"))
}

pub fn get_config_file() -> Result<PathBuf> {
    get_config_dir().map(|d| d.join("config"))
}

pub fn ensure_config_dir() -> Result<()> {
    let config_dir = get_config_dir()?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    Ok(())
}

pub fn ensure_crust_dir() -> Result<()> {
    ensure_config_dir()
}

pub fn load_credentials() -> Result<CredentialsFile> {
    let creds_file = get_credentials_file()?;

    if !creds_file.exists() {
        return Ok(CredentialsFile {
            credentials: Vec::new(),
        });
    }

    let contents = fs::read_to_string(&creds_file)?;
    let creds: CredentialsFile = serde_json::from_str(&contents)?;
    Ok(creds)
}

pub fn save_credentials(creds_file: &CredentialsFile) -> Result<()> {
    ensure_config_dir()?;
    let creds_path = get_credentials_file()?;
    let json = serde_json::to_string_pretty(&creds_file)?;
    fs::write(&creds_path, json)?;
    Ok(())
}

#[allow(dead_code)]
pub fn find_credential(server: &str) -> Result<Option<Credentials>> {
    let creds_file = load_credentials()?;
    Ok(creds_file
        .credentials
        .iter()
        .find(|c| c.server == server)
        .cloned())
}

pub fn add_credential(server: &str, username: &str, token: &str, expires_at: &str) -> Result<()> {
    let mut creds_file = load_credentials()?;

    // Remove existing credential for this server
    creds_file.credentials.retain(|c| c.server != server);

    // Add new credential
    creds_file.credentials.push(Credentials {
        server: server.to_string(),
        username: username.to_string(),
        token: token.to_string(),
        expires_at: expires_at.to_string(),
    });

    save_credentials(&creds_file)?;
    Ok(())
}

pub fn remove_credential(server: &str) -> Result<()> {
    let mut creds_file = load_credentials()?;
    creds_file.credentials.retain(|c| c.server != server);
    save_credentials(&creds_file)?;
    Ok(())
}

#[allow(dead_code)]
pub fn get_all_servers() -> Result<Vec<String>> {
    let creds_file = load_credentials()?;
    Ok(creds_file
        .credentials
        .iter()
        .map(|c| c.server.clone())
        .collect())
}
