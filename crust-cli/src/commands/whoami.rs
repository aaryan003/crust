// whoami command - display current authenticated user and token expiration

use anyhow::{anyhow, Result};

use crate::client::CrustClient;
use crate::config;

pub fn cmd_whoami(server_url: Option<&str>) -> Result<()> {
    // Load all credentials
    let creds_file = config::load_credentials()?;

    if creds_file.credentials.is_empty() {
        return Err(anyhow!(
            "Not logged in. Use 'crust login <server>' to authenticate"
        ));
    }

    // Find the credential to use
    let credential = match server_url {
        Some(url) => {
            let server = url.trim_end_matches('/');
            creds_file
                .credentials
                .iter()
                .find(|c| c.server == server)
                .ok_or_else(|| anyhow!("Not logged in to {}", server))?
        }
        None => {
            // If only one credential, use it
            if creds_file.credentials.len() == 1 {
                &creds_file.credentials[0]
            } else {
                // Multiple credentials, ask user to specify
                return Err(anyhow!(
                    "Multiple servers configured. Please specify: crust whoami [server]"
                ));
            }
        }
    };

    // Verify token is still valid
    let client = CrustClient::with_token(credential.server.clone(), credential.token.clone());
    let user = client.get_current_user()?;

    println!("{} @ {}", user.username, credential.server);
    println!("Token expires at: {}", credential.expires_at);

    Ok(())
}
