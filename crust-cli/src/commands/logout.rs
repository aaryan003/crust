// logout command - remove credentials for a CRUST server

use anyhow::Result;

use crate::config;

pub fn cmd_logout(server_url: Option<&str>) -> Result<()> {
    let server = match server_url {
        Some(url) => url.trim_end_matches('/').to_string(),
        None => {
            // Try to find a default server from credentials
            let creds = config::load_credentials()?;
            if creds.credentials.is_empty() {
                println!("Not logged in (no credentials found)");
                return Ok(());
            }
            if creds.credentials.len() == 1 {
                creds.credentials[0].server.clone()
            } else {
                // Multiple credentials, ask user to specify
                return Err(anyhow::anyhow!(
                    "Multiple servers configured. Please specify: crust logout <server>"
                ));
            }
        }
    };

    config::remove_credential(&server)?;
    println!("Logged out from {}", server);
    Ok(())
}
