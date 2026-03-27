/// Authentication configuration for either Keycloak or Auth0.
///
/// Auto-detects the provider based on which credentials are available.
/// If `AUTH0_DOMAIN` env var is set, uses Auth0. Otherwise uses Keycloak.
pub enum AuthConfig {
    Keycloak {
        client_id: String,
        username: String,
        password: String,
        url: String,
    },
    Auth0 {
        client_id: String,
        client_secret: String,
        audience: String,
        url: String,
    },
}

/// Unified authentication response.
pub struct AuthResponse {
    pub access_token: String,
    pub expires_in: u32,
    /// Only available with Keycloak password flow
    pub refresh_token: Option<String>,
}

impl AuthConfig {
    /// Authenticate and return a token.
    pub async fn authenticate(&self) -> Result<AuthResponse, String> {
        match self {
            AuthConfig::Keycloak {
                client_id,
                username,
                password,
                url,
            } => {
                let response =
                    keycloak::login::password(keycloak::login::PasswordParams {
                        client_id: client_id.clone(),
                        username: username.clone(),
                        password: password.clone(),
                        url: url.clone(),
                    })
                    .await
                    .map_err(|e| format!("Keycloak authentication failed: {e}"))?;

                Ok(AuthResponse {
                    access_token: response.access_token,
                    expires_in: response.expires_in,
                    refresh_token: Some(response.refresh_token),
                })
            }
            AuthConfig::Auth0 {
                client_id,
                client_secret,
                audience,
                url,
            } => {
                let response =
                    auth0::login::client_credentials(auth0::login::ClientCredentialsParams {
                        client_id: client_id.clone(),
                        client_secret: client_secret.clone(),
                        audience: audience.clone(),
                        url: url.clone(),
                    })
                    .await
                    .map_err(|e| format!("Auth0 authentication failed: {e}"))?;

                Ok(AuthResponse {
                    access_token: response.access_token,
                    expires_in: response.expires_in,
                    refresh_token: None,
                })
            }
        }
    }

    /// Refresh the token. For Keycloak, uses refresh_token flow with password
    /// fallback. For Auth0, re-authenticates (no refresh tokens in M2M flow).
    pub async fn refresh(
        &self,
        refresh_token: Option<&str>,
    ) -> Result<AuthResponse, String> {
        match self {
            AuthConfig::Keycloak {
                client_id, url, ..
            } => {
                // Try refresh token first
                if let Some(rt) = refresh_token {
                    match keycloak::login::refresh(keycloak::login::RefreshParams {
                        client_id: client_id.clone(),
                        refresh_token: rt.to_string(),
                        url: url.clone(),
                    })
                    .await
                    {
                        Ok(response) => {
                            return Ok(AuthResponse {
                                access_token: response.access_token,
                                expires_in: response.expires_in,
                                refresh_token: Some(response.refresh_token),
                            });
                        }
                        Err(e) => {
                            if !e.contains("Token is not active") {
                                return Err(format!("Failed to refresh JWT: {e}"));
                            }
                            // Fall through to password login
                        }
                    }
                }
                // Fallback to password login
                self.authenticate().await
            }
            AuthConfig::Auth0 { .. } => {
                // Auth0 M2M has no refresh tokens — just re-authenticate
                self.authenticate().await
            }
        }
    }

    /// Build from environment variables. Auto-detects provider.
    ///
    /// If `AUTH0_DOMAIN` is set, uses Auth0 (requires `AUTH0_CLIENT_ID`,
    /// `AUTH0_CLIENT_SECRET`, `AUTH0_AUDIENCE`).
    ///
    /// Otherwise uses Keycloak (requires `KEYCLOAK_HOST`, `KEYCLOAK_REALM`,
    /// `KEYCLOAK_CLIENT_ID`, `KEYCLOAK_USERNAME`, `KEYCLOAK_PASSWORD`).
    pub fn from_env() -> Result<Self, String> {
        if let Ok(auth0_domain) = std::env::var("AUTH0_DOMAIN") {
            Ok(AuthConfig::Auth0 {
                url: auth0::login::auth0_url(&auth0_domain),
                client_id: std::env::var("AUTH0_CLIENT_ID")
                    .map_err(|_| "AUTH0_CLIENT_ID must be set")?,
                client_secret: std::env::var("AUTH0_CLIENT_SECRET")
                    .map_err(|_| "AUTH0_CLIENT_SECRET must be set")?,
                audience: std::env::var("AUTH0_AUDIENCE")
                    .map_err(|_| "AUTH0_AUDIENCE must be set")?,
            })
        } else {
            Ok(AuthConfig::Keycloak {
                url: keycloak::login::password_url(
                    &std::env::var("KEYCLOAK_HOST")
                        .map_err(|_| "KEYCLOAK_HOST must be set")?,
                    &std::env::var("KEYCLOAK_REALM")
                        .map_err(|_| "KEYCLOAK_REALM must be set")?,
                ),
                client_id: std::env::var("KEYCLOAK_CLIENT_ID")
                    .map_err(|_| "KEYCLOAK_CLIENT_ID must be set")?,
                username: std::env::var("KEYCLOAK_USERNAME")
                    .map_err(|_| "KEYCLOAK_USERNAME must be set")?,
                password: std::env::var("KEYCLOAK_PASSWORD")
                    .map_err(|_| "KEYCLOAK_PASSWORD must be set")?,
            })
        }
    }
}
