// login command - authenticate with a CRUST server and store credentials

use anyhow::Result;
use rpassword::read_password;
use std::io::{self, Write};

use crate::client::CrustClient;
use crate::config;

pub fn cmd_login(server_url: &str, username_arg: Option<&str>, password_arg: Option<&str>) -> Result<()> {
    // Normalize server URL (remove trailing slash)
    let server = server_url.trim_end_matches('/');

    // Verify server is reachable
    let client = CrustClient::new(server.to_string());
    client.verify_server_reachable()?;

    // Get username — from flag or interactive prompt
    let username_owned: String;
    let username: &str = if let Some(u) = username_arg {
        u
    } else {
        print!("Username: ");
        io::stdout().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        username_owned = buf.trim().to_string();
        &username_owned
    };

    if username.is_empty() {
        return Err(anyhow::anyhow!("Username cannot be empty"));
    }

    // Get password — from flag or hidden interactive prompt
    let password: String = if let Some(p) = password_arg {
        p.to_string()
    } else {
        print!("Password: ");
        io::stdout().flush()?;
        read_password()?
    };

    if password.is_empty() {
        return Err(anyhow::anyhow!("Password cannot be empty"));
    }

    // Attempt login
    let login_data = client.login(username, &password)?;

    // Store credentials
    config::add_credential(server, username, &login_data.token, &login_data.expires_at)?;

    println!("Logged in as {} on {}", username, server);
    Ok(())
}
