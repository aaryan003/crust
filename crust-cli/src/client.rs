// HTTP client module - handles communication with CRUST server

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub data: Option<LoginData>,
    pub error: Option<ErrorResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginData {
    pub user: UserInfo,
    pub token: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ErrorResponse>,
}

pub struct CrustClient {
    server_url: String,
    token: Option<String>,
}

impl CrustClient {
    pub fn new(server_url: String) -> Self {
        CrustClient {
            server_url,
            token: None,
        }
    }

    pub fn with_token(server_url: String, token: String) -> Self {
        CrustClient {
            server_url,
            token: Some(token),
        }
    }

    /// Login to the server with username and password
    pub fn login(&self, username: &str, password: &str) -> Result<LoginData> {
        let client = reqwest::blocking::Client::new();
        let url = format!("{}/api/v1/auth/login", self.server_url);

        let request = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| anyhow!("Network error: {}", e))?;

        let _status = response.status();
        let body: LoginResponse = response
            .json()
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        if !body.success {
            let error = body.error.ok_or_else(|| anyhow!("Unknown error"))?;
            return Err(anyhow!("{}: {}", error.code, error.message));
        }

        let data = body
            .data
            .ok_or_else(|| anyhow!("No login data in response"))?;
        Ok(data)
    }

    /// Check authentication by fetching current user
    pub fn get_current_user(&self) -> Result<UserInfo> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| anyhow!("No token available"))?;

        let client = reqwest::blocking::Client::new();
        let url = format!("{}/api/v1/auth/me", self.server_url);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .map_err(|e| anyhow!("Network error: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!("Authentication failed: {}", status));
        }

        #[derive(Debug, Serialize, Deserialize)]
        struct UserResponse {
            success: bool,
            data: Option<UserInfo>,
        }

        let body: UserResponse = response
            .json()
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        body.data.ok_or_else(|| anyhow!("No user data in response"))
    }

    pub fn verify_server_reachable(&self) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let url = format!("{}/health", self.server_url);

        client
            .get(&url)
            .send()
            .map_err(|e| anyhow!("Could not reach server: {}", e))?;

        Ok(())
    }

    /// POST JSON request with authentication
    pub fn post_json<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        request: &T,
        token: &Option<String>,
    ) -> Result<R> {
        let client = reqwest::blocking::Client::new();
        let mut req = client.post(url);

        if let Some(token) = token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req
            .json(request)
            .send()
            .map_err(|e| anyhow!("Network error: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!("Request failed with status {}", status));
        }

        let body: ApiResponse<R> = response
            .json()
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        if !body.success {
            let error = body.error.ok_or_else(|| anyhow!("Unknown error"))?;
            return Err(anyhow!("{}: {}", error.code, error.message));
        }

        body.data.ok_or_else(|| anyhow!("No data in response"))
    }

    /// GET request with optional auth, parse JSON ApiResponse<R>
    pub fn get_json<R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        token: &Option<String>,
    ) -> Result<R> {
        let client = reqwest::blocking::Client::new();
        let mut req = client.get(url);

        if let Some(token) = token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req.send().map_err(|e| anyhow!("Network error: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().unwrap_or_default();
            return Err(anyhow!("Request failed with status {}: {}", status, body_text));
        }

        let body: ApiResponse<R> = response
            .json()
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        if !body.success {
            let error = body.error.ok_or_else(|| anyhow!("Unknown error"))?;
            return Err(anyhow!("{}: {}", error.code, error.message));
        }

        body.data.ok_or_else(|| anyhow!("No data in response"))
    }

    /// POST JSON request, receive raw binary response
    pub fn post_json_raw<T: Serialize>(
        &self,
        url: &str,
        request: &T,
        token: &Option<String>,
    ) -> Result<Vec<u8>> {
        let client = reqwest::blocking::Client::new();
        let mut req = client.post(url);

        if let Some(token) = token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req
            .json(request)
            .send()
            .map_err(|e| anyhow!("Network error: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().unwrap_or_default();
            return Err(anyhow!("Request failed with status {}: {}", status, body_text));
        }

        response
            .bytes()
            .map_err(|e| anyhow!("Failed to read response: {}", e))
            .map(|b| b.to_vec())
    }

    /// POST binary data and parse JSON ApiResponse<T>
    pub fn post_binary_json<R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        data: Vec<u8>,
        token: &Option<String>,
    ) -> Result<R> {
        let client = reqwest::blocking::Client::new();
        let mut req = client.post(url);

        if let Some(token) = token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req
            .body(data)
            .header("Content-Type", "application/octet-stream")
            .send()
            .map_err(|e| anyhow!("Network error: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().unwrap_or_default();
            return Err(anyhow!("Request failed with status {}: {}", status, body_text));
        }

        let body: ApiResponse<R> = response
            .json()
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        if !body.success {
            let error = body.error.ok_or_else(|| anyhow!("Unknown error"))?;
            return Err(anyhow!("{}: {}", error.code, error.message));
        }

        body.data.ok_or_else(|| anyhow!("No data in response"))
    }

    /// POST binary data with authentication
    #[allow(dead_code)]
    pub fn post_binary(
        &self,
        url: &str,
        data: Vec<u8>,
        token: &Option<String>,
    ) -> Result<serde_json::Value> {
        let client = reqwest::blocking::Client::new();
        let mut req = client.post(url);

        if let Some(token) = token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req
            .body(data)
            .header("Content-Type", "application/octet-stream")
            .send()
            .map_err(|e| anyhow!("Network error: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!("Request failed with status {}", status));
        }

        response
            .json()
            .map_err(|e| anyhow!("Failed to parse response: {}", e))
    }

    /// GET binary data with authentication
    pub fn get_raw(&self, url: &str, token: &Option<String>) -> Result<Vec<u8>> {
        let client = reqwest::blocking::Client::new();
        let mut req = client.get(url);

        if let Some(token) = token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req.send().map_err(|e| anyhow!("Network error: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!("Request failed with status {}", status));
        }

        response
            .bytes()
            .map_err(|e| anyhow!("Failed to read response: {}", e))
            .map(|b| b.to_vec())
    }
}
