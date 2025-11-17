use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Request {
    #[serde(rename = "choiceArguments")]
    pub choice_arguments: common::transfer_factory::ChoiceArguments,
    #[serde(rename = "excludeDebugFields")]
    pub exclude_debug_fields: bool,
}

pub struct Params {
    pub registry_url: String,
    pub decentralized_party_id: String,
    pub request: Request,
}

pub async fn get(params: Params) -> Result<common::transfer_factory::Response, String> {
    let client = reqwest::Client::new();

    let url = format!(
        "{}/api/token-standard/v0/registrars/{}/registry/transfer-instruction/v1/transfer-factory",
        params.registry_url, params.decentralized_party_id
    );
    let response = client
        .post(url.to_string())
        .json(&params.request)
        .send()
        .await
        .map_err(|e| format!("{}", e))?;

    let status = response.status();
    let body_raw = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "Transfer factory request failed [{}]: {:?}",
            status, body_raw
        ));
    }

    let body: common::transfer_factory::Response =
        serde_json::from_str(&body_raw).map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts;
    use keycloak::login::{PasswordParams, password, password_url};
    use std::collections::HashMap;
    use std::env;
    use std::ops::Add;

    #[tokio::test]
    async fn test_transfer_factory() {
        dotenvy::dotenv().ok();

        let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
        let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");

        let params = PasswordParams {
            client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
            username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
            password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
            url: password_url(
                &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
                &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
            ),
        };
        let login_response = password(params).await.unwrap();

        let contracts = cbtc::active_contracts::get(cbtc::active_contracts::Params {
            ledger_host: ledger_host.to_string(),
            party: party_id,
            access_token: login_response.access_token,
        })
        .await
        .unwrap();

        let mut input_contract_ids: Vec<String> = Vec::new();
        for contract in &contracts {
            input_contract_ids.push(contract.created_event.contract_id.clone());
        }

        let mut transfer_meta: HashMap<String, String> = HashMap::new();
        transfer_meta.insert(
            "splice.lfdecentralizedtrust.org/reason".to_string(),
            "".to_string(),
        );

        let params = Params {
            registry_url: consts::DEVNET_REGISTRY_URL.to_string(),
            decentralized_party_id: common::consts::DEVNET_DECENTRALIZED_PARTY_ID.to_string(),
            request: Request {
                choice_arguments: common::transfer_factory::ChoiceArguments {
                    expected_admin: common::consts::DEVNET_DECENTRALIZED_PARTY_ID.to_string(),
                    transfer: common::transfer::Transfer {
                        sender: env::var("PARTY_ID").expect("PARTY_ID must be set"),
                        receiver: env::var("LIB_TEST_RECEIVER_PARTY_ID")
                            .expect("LIB_TEST_RECEIVER_PARTY_ID must be set"),
                        amount: "0.02".to_string(),
                        instrument_id: common::transfer::InstrumentId {
                            admin: common::consts::DEVNET_DECENTRALIZED_PARTY_ID.to_string(),
                            id: "CBTC".to_string(),
                        },
                        requested_at: chrono::Utc::now().to_rfc3339(),
                        execute_before: chrono::Utc::now()
                            .add(chrono::Duration::hours(5))
                            .to_rfc3339(),
                        input_holding_cids: Some(input_contract_ids),
                        meta: Some(common::transfer::Meta {
                            values: Some(transfer_meta),
                        }),
                    },
                    extra_args: common::transfer_factory::ExtraArgs {
                        context: common::transfer_factory::Context {
                            values: HashMap::new(),
                        },
                        meta: common::transfer_factory::Meta {
                            values: common::transfer_factory::MetaValue {},
                        },
                    },
                },
                exclude_debug_fields: true,
            },
        };

        let _result = get(params).await.unwrap();
    }
}
