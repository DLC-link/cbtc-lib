//! Auth0 Authentication Module
//!
//! This module provides Auth0 client credentials authentication for Canton network access.
//! It's a local implementation that works alongside the external `keycloak` crate,
//! allowing you to use Auth0 instead of Keycloak for authentication.
//!
//! # Usage
//!
//! ```rust,ignore
//! use cbtc::auth0::{client_credentials, ClientCredentialsParams, auth0_url};
//!
//! let params = ClientCredentialsParams {
//!     url: auth0_url("https://your-tenant.auth0.com"),
//!     client_id: "your-client-id".to_string(),
//!     client_secret: "your-client-secret".to_string(),
//!     audience: "https://your-api-audience".to_string(),
//! };
//!
//! let auth = client_credentials(params).await?;
//! // Use auth.access_token for API calls
//! ```

use base64::Engine;
use serde::Deserialize;

/// Parameters for Auth0 client credentials authentication
pub struct ClientCredentialsParams {
    /// The Auth0 token endpoint URL (use `auth0_url()` to construct)
    pub url: String,
    /// Your Auth0 application's client ID
    pub client_id: String,
    /// Your Auth0 application's client secret
    pub client_secret: String,
    /// The API audience identifier
    pub audience: String,
}

/// Authentication response containing the access token
#[derive(Deserialize, Debug, Clone)]
pub struct Response {
    /// The JWT access token to use for API requests
    pub access_token: String,
    /// Token expiration time in seconds
    #[serde(default)]
    pub expires_in: u32,
    /// Token type (usually "Bearer")
    #[serde(default)]
    pub token_type: String,
}

impl Response {
    /// Extract the user ID (subject claim) from the access token JWT
    ///
    /// Returns the 'sub' claim which is typically the Auth0 user/client identifier.
    /// For machine-to-machine tokens, this is usually `client_id@clients`.
    ///
    /// # Errors
    ///
    /// Returns an error if the JWT is malformed or doesn't contain a 'sub' claim.
    pub fn get_user_id(&self) -> Result<String, String> {
        // JWT format: header.payload.signature
        let parts: Vec<&str> = self.access_token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT format".to_string());
        }

        // Decode the payload (second part)
        let payload = parts[1];

        // URL-safe base64 without padding - we need to add padding for the decoder
        let padding_needed = (4 - (payload.len() % 4)) % 4;
        let padded = if padding_needed > 0 {
            format!("{}{}", payload, "=".repeat(padding_needed))
        } else {
            payload.to_string()
        };

        // Decode base64 - use URL_SAFE engine first, fall back to STANDARD
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload)
            .or_else(|_| {
                base64::engine::general_purpose::STANDARD.decode(&padded)
            })
            .map_err(|e| format!("Failed to decode JWT payload: {}", e))?;

        // Parse JSON
        let json: serde_json::Value = serde_json::from_slice(&decoded)
            .map_err(|e| format!("Failed to parse JWT payload JSON: {}", e))?;

        // Extract 'sub' claim
        json.get("sub")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "JWT does not contain 'sub' claim".to_string())
    }

    /// Extract an arbitrary claim from the access token JWT
    ///
    /// Useful for extracting custom claims like party_id, roles, etc.
    pub fn get_claim(&self, claim_name: &str) -> Result<serde_json::Value, String> {
        let parts: Vec<&str> = self.access_token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT format".to_string());
        }

        let payload = parts[1];
        let padding_needed = (4 - (payload.len() % 4)) % 4;
        let padded = if padding_needed > 0 {
            format!("{}{}", payload, "=".repeat(padding_needed))
        } else {
            payload.to_string()
        };

        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload)
            .or_else(|_| {
                base64::engine::general_purpose::STANDARD.decode(&padded)
            })
            .map_err(|e| format!("Failed to decode JWT payload: {}", e))?;

        let json: serde_json::Value = serde_json::from_slice(&decoded)
            .map_err(|e| format!("Failed to parse JWT payload JSON: {}", e))?;

        json.get(claim_name)
            .cloned()
            .ok_or_else(|| format!("JWT does not contain '{}' claim", claim_name))
    }
}

/// Perform Auth0 client credentials authentication
///
/// This function exchanges client credentials for an access token using Auth0's
/// OAuth 2.0 client credentials flow. The returned token can be used to
/// authenticate with Canton APIs.
///
/// # Arguments
///
/// * `params` - Authentication parameters including URL, client_id, client_secret, and audience
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - Auth0 returns an error response
/// - The response cannot be parsed
///
/// # Example
///
/// ```rust,ignore
/// let auth = client_credentials(ClientCredentialsParams {
///     url: auth0_url("https://your-tenant.auth0.com"),
///     client_id: "abc123".to_string(),
///     client_secret: "secret".to_string(),
///     audience: "https://api.example.com".to_string(),
/// }).await?;
/// ```
pub async fn client_credentials(params: ClientCredentialsParams) -> Result<Response, String> {
    let client = reqwest::Client::new();

    // Auth0 uses JSON body for client credentials
    let json_body = serde_json::json!({
        "grant_type": "client_credentials",
        "client_id": params.client_id,
        "client_secret": params.client_secret,
        "audience": params.audience,
    });

    let res = client
        .post(&params.url)
        .json(&json_body)
        .send()
        .await
        .map_err(|e| format!("Auth0 client_credentials request failed: {}", e))?;

    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("Failed to read Auth0 response: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "Auth0 authentication failed [{}]: {}",
            status, body
        ));
    }

    let response: Response = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Auth0 response: {} - body: {}", e, body))?;

    Ok(response)
}

/// Construct Auth0 OAuth token endpoint URL
///
/// # Arguments
///
/// * `domain` - Your Auth0 domain (e.g., "https://your-tenant.auth0.com")
///
/// # Returns
///
/// The full token endpoint URL (e.g., "https://your-tenant.auth0.com/oauth/token")
pub fn auth0_url(domain: &str) -> String {
    let domain = domain.trim_end_matches('/');
    format!("{}/oauth/token", domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth0_url() {
        assert_eq!(
            auth0_url("https://example.auth0.com"),
            "https://example.auth0.com/oauth/token"
        );
        assert_eq!(
            auth0_url("https://example.auth0.com/"),
            "https://example.auth0.com/oauth/token"
        );
    }
}

