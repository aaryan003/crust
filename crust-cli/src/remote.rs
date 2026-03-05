// Remote synchronization logic - handles fetch, push, and clone

use crate::client::CrustClient;
use crate::config::find_credential;
use anyhow::{anyhow, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PreflightRequest {
    pub wants: Vec<String>,
    pub haves: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreflightResponse {
    pub wants: Vec<String>,
    pub haves: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchRequest {
    pub wants: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub objects_stored: usize,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefUpdate {
    pub ref_name: String,
    pub old_sha: String,
    pub new_sha: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefUpdateRequest {
    pub updates: Vec<RefUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefUpdateResult {
    pub ref_name: String,
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
}

pub struct RemoteSync {
    client: CrustClient,
    server_url: String,
    owner: String,
    repo: String,
    token: Option<String>,
}

impl RemoteSync {
    pub fn new(server_url: &str, owner: &str, repo: &str) -> Result<Self> {
        // Extract base server URL (scheme + host + port) for credential lookup.
        // server_url may be a full repo URL like "http://host:8080/owner/repo"
        // or a base URL like "http://host:8080".
        let base_server_url = extract_server_base(server_url);

        // Find credential for this base server
        let token = find_credential(&base_server_url)?.map(|c| c.token);

        let client = if let Some(ref t) = token {
            CrustClient::with_token(base_server_url.clone(), t.clone())
        } else {
            CrustClient::new(base_server_url.clone())
        };

        Ok(RemoteSync {
            client,
            server_url: base_server_url,
            owner: owner.to_string(),
            repo: repo.to_string(),
            token,
        })
    }

    pub fn check_repo_exists(&self) -> Result<()> {
        let url = format!(
            "{}/api/v1/repos/{}/{}",
            self.server_url, self.owner, self.repo
        );
        // get_json will return Err if 404 or other non-2xx status
        let _: serde_json::Value = self.client.get_json(&url, &self.token)?;
        Ok(())
    }

    /// Returns the authentication token if one is loaded.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub fn fetch(&self, wants: Vec<String>, _haves: Vec<String>) -> Result<Vec<u8>> {
        if self.token.is_none() {
            return Err(anyhow!(
                "CLI_NOT_AUTHENTICATED: Not logged in to {}",
                self.server_url
            ));
        }

        let url = format!(
            "{}/api/v1/repos/{}/{}/objects/fetch",
            self.server_url, self.owner, self.repo
        );

        let request = FetchRequest { wants };

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{msg} {spinner}")
                .unwrap(),
        );
        pb.set_message("Downloading objects...");

        // POST JSON body, receive raw CRUSTPACK binary
        let response = self.client.post_json_raw(&url, &request, &self.token)?;

        pb.finish_with_message("Downloaded objects");

        Ok(response)
    }

    pub fn upload(&self, pack_data: Vec<u8>) -> Result<UploadResponse> {
        if self.token.is_none() {
            return Err(anyhow!(
                "CLI_NOT_AUTHENTICATED: Not logged in to {}",
                self.server_url
            ));
        }

        let url = format!(
            "{}/api/v1/repos/{}/{}/objects/upload",
            self.server_url, self.owner, self.repo
        );

        let pb = ProgressBar::new(pack_data.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes}")
                .unwrap(),
        );
        pb.set_message("Uploading objects");

        // Upload raw CRUSTPACK binary data
        let response = self.client.post_binary_json::<UploadResponse>(&url, pack_data, &self.token)?;

        pb.finish_with_message("Uploaded objects");

        Ok(response)
    }

    pub fn update_refs(&self, updates: Vec<RefUpdate>) -> Result<Vec<RefUpdateResult>> {
        if self.token.is_none() {
            return Err(anyhow!(
                "CLI_NOT_AUTHENTICATED: Not logged in to {}",
                self.server_url
            ));
        }

        let url = format!(
            "{}/api/v1/repos/{}/{}/refs/update",
            self.server_url, self.owner, self.repo
        );

        let request = RefUpdateRequest { updates };

        let response = self.client.post_json(&url, &request, &self.token)?;

        Ok(response)
    }

    /// GET /api/v1/repos/:owner/:repo/refs — returns heads map (branch → commit sha)
    pub fn get_refs(&self) -> Result<std::collections::HashMap<String, String>> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/refs",
            self.server_url, self.owner, self.repo
        );

        #[derive(serde::Deserialize)]
        struct RefsData {
            heads: std::collections::HashMap<String, String>,
        }

        let refs_data: RefsData = self.client.get_json(&url, &self.token)?;
        Ok(refs_data.heads)
    }

    pub fn preflight(&self, wants: Vec<String>, haves: Vec<String>) -> Result<PreflightResponse> {
        if self.token.is_none() {
            return Err(anyhow!(
                "CLI_NOT_AUTHENTICATED: Not logged in to {}",
                self.server_url
            ));
        }

        let url = format!(
            "{}/api/v1/repos/{}/{}/refs/preflight",
            self.server_url, self.owner, self.repo
        );

        let request = PreflightRequest { wants, haves };

        let response = self.client.post_json(&url, &request, &self.token)?;

        Ok(response)
    }
}

/// Extract the base server URL (scheme + host + optional port) from a full URL.
/// "http://localhost:8080/alice/repo" → "http://localhost:8080"
/// "http://localhost:8080" → "http://localhost:8080"
fn extract_server_base(url: &str) -> String {
    // Find the scheme separator
    if let Some(after_scheme) = url.find("://") {
        let rest = &url[after_scheme + 3..]; // skip "://"
        // Find the first '/' after scheme+host (the path start)
        if let Some(slash_pos) = rest.find('/') {
            format!("{}://{}", &url[..after_scheme], &rest[..slash_pos])
        } else {
            // No path, URL is already a base
            url.to_string()
        }
    } else {
        url.to_string()
    }
}

#[allow(dead_code)]
pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb
}
