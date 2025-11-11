use serde::Deserialize;

pub struct ClientCredentialsParams {
    pub url: String,
    pub client_id: String,
    pub client_secret: String,
}

pub struct PasswordParams {
    pub client_id: String,
    pub username: String,
    pub password: String,
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct Response {
    pub access_token: String,
    #[serde(default)]
    pub expires_in: u32,
    #[serde(default)]
    pub refresh_token: String,
}

pub struct RefreshParams {
    pub client_id: String,
    pub refresh_token: String,
    pub url: String,
}

pub async fn client_credentials(params: ClientCredentialsParams) -> Result<Response, String> {
    let client = reqwest::Client::new();
    let form = [
        ("grant_type", "client_credentials"),
        ("client_id", &*params.client_id),
        ("client_secret", &*params.client_secret),
    ];

    let res = client
        .post(params.url)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Keycloak client_credentials login request error: {}", e))?;

    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("Failed to read response (client_credentials): {}", e))?;
    if !status.is_success() {
        return Err(format!(
            "Failed to get token (client_credentials) [{}]: {}",
            status, body
        ));
    }
    let response: Response = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response (client_credentials): {}", e))?;

    Ok(response)
}

pub async fn password(params: PasswordParams) -> Result<Response, String> {
    let client = reqwest::Client::new();
    let form = [
        ("grant_type", "password"),
        ("client_id", &*params.client_id),
        ("username", &*params.username),
        ("password", &*params.password),
    ];
    let res = client
        .post(params.url)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Keycloak password login request error: {}", e))?;

    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    if !status.is_success() {
        return Err(format!(
            "Failed to get token (password) [{}]: {}",
            status, body
        ));
    }
    let response: Response = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response (password): {}", e))?;

    Ok(response)
}

pub fn client_credentials_url(host: &str, realm: &str) -> String {
    format!(
        "{}/auth/realms/{}/protocol/openid-connect/token",
        host, realm
    )
}

pub fn password_url(host: &str, realm: &str) -> String {
    format!(
        "{}/auth/realms/{}/protocol/openid-connect/token",
        host, realm
    )
}

pub fn password_master_url(host: &str) -> String {
    format!("{}/auth/realms/master/protocol/openid-connect/token", host)
}

pub async fn refresh(params: RefreshParams) -> Result<Response, String> {
    let client = reqwest::Client::new();
    let form = [
        ("grant_type", "refresh_token"),
        ("client_id", &*params.client_id),
        ("refresh_token", &*params.refresh_token),
    ];

    let res = client
        .post(params.url)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Keycloak refresh token request error: {}", e))?;

    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("Failed to read response (refresh): {}", e))?;
    if !status.is_success() {
        return Err(format!(
            "Failed to refresh token [{}]: {}",
            status, body
        ));
    }
    let response: Response = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response (refresh): {}", e))?;

    Ok(response)
}
