// Semantic accessors for `canton_api_client::models::Event` (re-exported as
// `ledger::models::Event`).
//
// The OpenAPI generator names the `oneOf` variants by position, because the
// upstream Canton spec gives the branches no names. `EventOneOf1` is the
// created event. Match on those positional names here and nowhere else, so a
// regenerated spec touches one file.

use ledger::models::{CreatedEvent, Event};

pub(crate) fn as_created_event(event: &Event) -> Option<&CreatedEvent> {
    match event {
        Event::EventOneOf1(wrapper) => Some(&wrapper.created_event),
        _ => None,
    }
}
