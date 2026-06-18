use std::time::Duration;

use ratatui::crossterm::event::{self, Event as CtEvent, KeyCode, KeyEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{Event, KeyKind};
use crate::config::Profile;
use crate::ops::{self, OpContext, Operation};
use crate::session;

/// Map a raw key code to a semantic `KeyKind`, or `None` to ignore it.
pub fn map_key(code: KeyCode) -> Option<KeyKind> {
    match code {
        KeyCode::Up => Some(KeyKind::Up),
        KeyCode::Down => Some(KeyKind::Down),
        KeyCode::Enter => Some(KeyKind::Enter),
        KeyCode::Tab => Some(KeyKind::Tab),
        KeyCode::PageUp => Some(KeyKind::PageUp),
        KeyCode::PageDown => Some(KeyKind::PageDown),
        KeyCode::Esc => Some(KeyKind::Esc),
        KeyCode::Char('q') => Some(KeyKind::Quit),
        KeyCode::Char('p') => Some(KeyKind::OpenParties),
        KeyCode::Char('P') => Some(KeyKind::OpenProfiles),
        KeyCode::Char('r') => Some(KeyKind::Refresh),
        _ => None,
    }
}

/// Spawn a blocking thread that reads terminal input and forwards mapped keys.
pub fn spawn_input_reader(tx: UnboundedSender<Event>) {
    std::thread::spawn(move || loop {
        if event::poll(Duration::from_millis(200)).unwrap_or(false) {
            if let Ok(CtEvent::Key(key)) = event::read()
                && key.kind == KeyEventKind::Press
                && let Some(k) = map_key(key.code)
                && tx.send(Event::Key(k)).is_err()
            {
                break;
            }
        } else if tx.is_closed() {
            break;
        }
    });
}

/// Spawn an async task that runs `op` and sends the result back. A panic in the
/// task is converted into an `Event::OpResult(Err(..))` so the UI never hangs.
pub fn spawn_op(tx: UnboundedSender<Event>, op: Operation, ctx: OpContext) {
    let panic_tx = tx.clone();
    let handle = tokio::spawn(async move {
        tracing::info!("running {op} as party {}", ctx.party);
        let result = ops::run(op, &ctx).await.map_err(|e| e.to_string());
        match &result {
            Ok(crate::ops::OpResult::Table { rows, .. }) => {
                tracing::info!("{op}: {} row(s) for party {}", rows.len(), ctx.party);
            }
            Ok(crate::ops::OpResult::Text { .. }) => {
                tracing::info!("{op}: text result for party {}", ctx.party);
            }
            Err(e) => tracing::warn!("{op} failed for party {}: {e}", ctx.party),
        }
        let _ = tx.send(Event::OpResult(result));
    });
    tokio::spawn(async move {
        if let Err(join_err) = handle.await {
            let _ = panic_tx.send(Event::OpResult(Err(format!("operation panicked: {join_err}"))));
        }
    });
}

/// Spawn an async task that logs in and fetches parties. A panic in the task is
/// converted into an `Event::LoginResult(Err(..))` so the UI never hangs.
pub fn spawn_login(tx: UnboundedSender<Event>, profile: Profile) {
    let panic_tx = tx.clone();
    let handle = tokio::spawn(async move {
        let result = async {
            let s = session::login(&profile).await.map_err(|e| e.to_string())?;
            let parties = session::fetch_parties(&s, &s.access_token)
                .await
                .map_err(|e| e.to_string())?;
            Ok::<_, String>((s.access_token, parties))
        }
        .await;
        match &result {
            Ok((_, parties)) => {
                tracing::info!(
                    "login ok for '{}': {} parties",
                    profile.keycloak_username,
                    parties.len()
                );
            }
            Err(e) => tracing::warn!("login failed for '{}': {e}", profile.keycloak_username),
        }
        let _ = tx.send(Event::LoginResult(result));
    });
    tokio::spawn(async move {
        if let Err(join_err) = handle.await {
            let _ = panic_tx.send(Event::LoginResult(Err(format!("login panicked: {join_err}"))));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_navigation_keys() {
        assert_eq!(map_key(KeyCode::Up), Some(crate::app::KeyKind::Up));
        assert_eq!(map_key(KeyCode::Enter), Some(crate::app::KeyKind::Enter));
        assert_eq!(map_key(KeyCode::Char('q')), Some(crate::app::KeyKind::Quit));
        assert_eq!(map_key(KeyCode::Char('p')), Some(crate::app::KeyKind::OpenParties));
        assert_eq!(map_key(KeyCode::Char('P')), Some(crate::app::KeyKind::OpenProfiles));
        assert_eq!(map_key(KeyCode::Esc), Some(crate::app::KeyKind::Esc));
        assert_eq!(map_key(KeyCode::Char('z')), None);
    }
}
