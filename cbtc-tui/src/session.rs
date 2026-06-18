use std::collections::BTreeMap;

use canton_api_client::apis::configuration::Configuration;
use canton_api_client::apis::default_api::get_v2_users_user_id_rights;
use canton_api_client::models::{Kind, ListUserRightsResponse};
use keycloak::login::{password, password_url, PasswordParams};

use crate::config::Profile;
use crate::error::{AppError, Result};

/// A party the logged-in user is entitled to act and/or read as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyRight {
    pub party: String,
    pub can_act_as: bool,
    pub can_read_as: bool,
}

/// An authenticated session for one profile.
#[derive(Debug, Clone)]
pub struct Session {
    pub access_token: String,
    pub user_id: String,
    pub ledger_host: String,
}

/// Aggregate a `ListUserRightsResponse` into per-party act/read flags.
pub fn parse_party_rights(resp: &ListUserRightsResponse) -> Vec<PartyRight> {
    let mut by_party: BTreeMap<String, PartyRight> = BTreeMap::new();
    let Some(rights) = resp.rights.as_ref() else {
        return Vec::new();
    };
    for right in rights {
        let Some(kind) = right.kind.as_deref() else {
            continue;
        };
        let (party, act, read) = match kind {
            Kind::KindOneOf(k) => (k.can_act_as.value.party.clone(), true, false),
            Kind::KindOneOf3(k) => (k.can_read_as.value.party.clone(), false, true),
            _ => continue,
        };
        let entry = by_party.entry(party.clone()).or_insert(PartyRight {
            party,
            can_act_as: false,
            can_read_as: false,
        });
        entry.can_act_as |= act;
        entry.can_read_as |= read;
    }
    by_party.into_values().collect()
}

/// Authenticate `profile` against Keycloak (password grant) and read the user id.
///
/// # Errors
/// Returns `AppError::Auth` on login or token-decode failure.
pub async fn login(profile: &Profile) -> Result<Session> {
    let url = password_url(&profile.keycloak_host, &profile.keycloak_realm);
    let resp = password(PasswordParams {
        client_id: profile.keycloak_client_id.clone(),
        username: profile.keycloak_username.clone(),
        password: profile.keycloak_password.clone(),
        url,
    })
    .await
    .map_err(AppError::Auth)?;
    let user_id = resp.get_user_id().map_err(AppError::Auth)?;
    Ok(Session {
        access_token: resp.access_token,
        user_id,
        ledger_host: profile.ledger_host.clone(),
    })
}

/// Fetch the parties `session.user_id` can act/read as from the ledger.
///
/// # Errors
/// Returns `AppError::Canton` on API failure.
pub async fn fetch_parties(session: &Session, token: &str) -> Result<Vec<PartyRight>> {
    let configuration = Configuration {
        base_path: session.ledger_host.clone(),
        bearer_access_token: Some(token.to_string()),
        ..Default::default()
    };
    let resp = get_v2_users_user_id_rights(&configuration, &session.user_id)
        .await
        .map_err(|e| AppError::Canton(e.to_string()))?;
    Ok(parse_party_rights(&resp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use canton_api_client::models::ListUserRightsResponse;

    #[test]
    fn parses_and_aggregates_party_rights() {
        // Arrange: real wire shape; Kind is #[serde(untagged)].
        let json = r#"{
            "rights": [
                {"kind": {"CanActAs":  {"value": {"party": "alice::1220ab"}}}},
                {"kind": {"CanReadAs": {"value": {"party": "alice::1220ab"}}}},
                {"kind": {"CanReadAs": {"value": {"party": "treasury::1220cd"}}}},
                {"kind": {"ParticipantAdmin": {"value": {}}}}
            ]
        }"#;
        let resp: ListUserRightsResponse = serde_json::from_str(json).unwrap();
        // Act
        let mut rights = parse_party_rights(&resp);
        rights.sort_by(|a, b| a.party.cmp(&b.party));
        // Assert
        assert_eq!(rights.len(), 2);
        assert_eq!(rights[0].party, "alice::1220ab");
        assert!(rights[0].can_act_as && rights[0].can_read_as);
        assert_eq!(rights[1].party, "treasury::1220cd");
        assert!(!rights[1].can_act_as && rights[1].can_read_as);
    }
}
