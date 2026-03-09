// remote command - manage remote repositories

use crate::config::Config;
use anyhow::{anyhow, Result};

pub fn cmd_remote_add(name: &str, url: &str) -> Result<()> {
    let mut config = Config::load()?;
    config.add_remote(name.to_string(), url.to_string())?;
    config.save()?;
    println!("Added remote '{}'", name);
    Ok(())
}

pub fn cmd_remote_list() -> Result<()> {
    let config = Config::load()?;
    let remotes = config.get_remotes();

    if remotes.is_empty() {
        println!("No remotes configured");
        return Ok(());
    }

    for (name, url) in remotes {
        println!("{:<12} {}", name, url);
    }
    Ok(())
}

pub fn cmd_remote_remove(name: &str) -> Result<()> {
    let mut config = Config::load()?;
    // Check remote exists
    if config.get_remote(name).is_none() {
        return Err(anyhow!("No such remote: '{}'", name));
    }
    config.delete_remote(name)?;
    config.save()?;
    println!("Removed remote '{}'", name);
    Ok(())
}

pub fn cmd_remote_rename(old_name: &str, new_name: &str) -> Result<()> {
    let mut config = Config::load()?;
    config.rename_remote(old_name, new_name)?;
    config.save()?;
    println!("Renamed remote '{}' -> '{}'", old_name, new_name);
    Ok(())
}

pub fn cmd_remote_set_url(name: &str, url: &str) -> Result<()> {
    let mut config = Config::load()?;
    config.set_remote_url(name, url)?;
    config.save()?;
    println!("Updated URL for remote '{}'", name);
    Ok(())
}
