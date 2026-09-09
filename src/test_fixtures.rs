//! Helpers for building typed `JsSubmitAndWaitForTransactionResponse`
//! values for parser unit tests. The canton-api-client model structs
//! reject responses that lack required fields like `nodeId`,
//! `createdAt`, `packageName`, `offset`, `synchronizerId`, etc. — these
//! helpers stamp in dummy values for those so each test fixture only
//! has to specify the fields it actually exercises.
use ledger::models::JsSubmitAndWaitForTransactionResponse;
use serde_json::{Value, json};

/// Build a flat-event `CreatedEvent` as a JSON value with required
/// structural fields filled in with placeholders. Pass `create_argument`
/// as `json!(null)` if the test doesn't care about it.
pub fn created_event_value(template_id: &str, contract_id: &str, create_argument: Value) -> Value {
    json!({
        "CreatedEvent": {
            "offset": 1_i64,
            "nodeId": 0_i32,
            "contractId": contract_id,
            "templateId": template_id,
            "createArgument": create_argument,
            "createdEventBlob": "",
            "witnessParties": [],
            "signatories": [],
            "observers": [],
            "createdAt": "1970-01-01T00:00:00Z",
            "packageName": "test-pkg",
            "representativePackageId": "test-pkg",
            "acsDelta": true,
        }
    })
}

/// Same as `created_event_value`, but lets the caller override
/// `createdEventBlob` — used by tests that assert on the blob being
/// propagated into the resulting domain object.
pub fn created_event_value_with_blob(
    template_id: &str,
    contract_id: &str,
    create_argument: Value,
    created_event_blob: &str,
) -> Value {
    let mut event = created_event_value(template_id, contract_id, create_argument);
    event["CreatedEvent"]["createdEventBlob"] = json!(created_event_blob);
    event
}

/// Build a flat-event `ExercisedEvent` as a JSON value with required
/// structural fields filled in with placeholders. Pass `exercise_result`
/// as `json!(null)` if the test doesn't care about it.
pub fn exercised_event_value(template_id: &str, choice: &str, exercise_result: Value) -> Value {
    json!({
        "ExercisedEvent": {
            "offset": 1_i64,
            "nodeId": 0_i32,
            "contractId": "00exercise-target",
            "templateId": template_id,
            "choice": choice,
            "choiceArgument": null,
            "actingParties": [],
            "consuming": true,
            "witnessParties": [],
            "lastDescendantNodeId": 0_i32,
            "exerciseResult": exercise_result,
            "packageName": "test-pkg",
            "acsDelta": true,
        }
    })
}

/// Build a `JsSubmitAndWaitForTransactionResponse` from an updateId and
/// an `events` value. Pass `json!(null)` to construct a response with an
/// empty events list (the typed model now treats `events` as required, so
/// "no events" is represented as `[]` rather than an absent field).
/// Deserializes through the typed model so fixtures fail loudly when the
/// shape diverges from canton-api-client's schema.
pub fn transaction_response(
    update_id: &str,
    events: Value,
) -> JsSubmitAndWaitForTransactionResponse {
    let events = if events.is_null() { json!([]) } else { events };
    let transaction = json!({
        "updateId": update_id,
        "commandId": "",
        "workflowId": "",
        "effectiveAt": "1970-01-01T00:00:00Z",
        "events": events,
        "offset": 1_i64,
        "synchronizerId": "test-synchronizer",
        "recordTime": "1970-01-01T00:00:00Z",
    });
    let envelope = json!({ "transaction": transaction });
    serde_json::from_value(envelope).expect("test fixture is not a valid response")
}
