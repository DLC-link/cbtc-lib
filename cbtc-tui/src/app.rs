use std::collections::HashMap;
use std::time::Instant;

use strum::IntoEnumIterator;

use crate::config::Config;
use crate::ops::{Operation, OpResult};
use crate::session::PartyRight;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Launcher,
    Main,
    PartyOverlay,
    Detail,
}

/// Which pane on the main screen has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Queries,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    Up,
    Down,
    Enter,
    Tab,
    PageUp,
    PageDown,
    OpenParties,
    OpenProfiles,
    Refresh,
    Quit,
    Esc,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Key(KeyKind),
    LoginResult(std::result::Result<(String, Vec<PartyRight>), String>),
    OpResult(std::result::Result<OpResult, String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Quit,
    Login(usize),
    RunOp(Operation),
    FetchParties,
}

/// A cached operation result with the instant it was fetched.
#[derive(Debug, Clone)]
pub struct CachedResult {
    pub result: OpResult,
    pub at: Instant,
}

/// Mutable UI state plus the pure transition function.
pub struct App {
    pub config: Config,
    pub screen: Screen,
    pub focus: Focus,
    pub selected_profile: usize,
    pub active_profile: Option<usize>,
    pub access_token: Option<String>,
    pub parties: Vec<PartyRight>,
    pub selected_party: usize,
    pub active_party: Option<String>,
    pub operations: Vec<Operation>,
    pub selected_op: usize,
    pub result: Option<OpResult>,
    pub result_selected: usize,
    pub result_at: Option<Instant>,
    pub cache: HashMap<(String, Operation), CachedResult>,
    /// (party, operation) currently being fetched, used to key the cache on completion.
    pub running: Option<(String, Operation)>,
    pub detail_lines: Vec<String>,
    pub detail_scroll: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub status: String,
}

impl App {
    pub fn new(config: Config) -> Self {
        App {
            config,
            screen: Screen::Launcher,
            focus: Focus::Queries,
            selected_profile: 0,
            active_profile: None,
            access_token: None,
            parties: Vec::new(),
            selected_party: 0,
            active_party: None,
            operations: Operation::iter().collect(),
            selected_op: 0,
            result: None,
            result_selected: 0,
            result_at: None,
            cache: HashMap::new(),
            running: None,
            detail_lines: Vec::new(),
            detail_scroll: 0,
            loading: false,
            error: None,
            status: String::new(),
        }
    }

    /// Apply an event, mutating state and returning side effects to run.
    pub fn update(&mut self, event: Event) -> Vec<Effect> {
        match event {
            Event::Key(key) => self.on_key(key),
            Event::LoginResult(Ok((token, parties))) => {
                self.access_token = Some(token);
                // Preserve the party the user was on (or their last saved choice)
                // across re-login; only fall back to the first party if it's gone.
                let preferred = self.active_party.clone().or_else(|| {
                    self.active_profile
                        .and_then(|i| self.config.profiles.get(i))
                        .and_then(|p| p.last_selected_party.clone())
                });
                let idx = preferred
                    .as_deref()
                    .and_then(|want| parties.iter().position(|p| p.party == want))
                    .unwrap_or(0);
                self.selected_party = idx;
                self.active_party = parties.get(idx).map(|p| p.party.clone());
                self.parties = parties;
                self.screen = Screen::Main;
                self.focus = Focus::Queries;
                self.error = None;
                self.status = "Logged in".to_string();
                self.show_cached_for_selected();
                Vec::new()
            }
            Event::LoginResult(Err(e)) => {
                self.error = Some(e);
                self.status = "Login failed".to_string();
                Vec::new()
            }
            Event::OpResult(Ok(result)) => {
                self.loading = false;
                self.error = None;
                let at = Instant::now();
                if let Some(key) = self.running.take() {
                    self.cache
                        .insert(key, CachedResult { result: result.clone(), at });
                }
                self.result = Some(result);
                self.result_at = Some(at);
                self.result_selected = 0;
                Vec::new()
            }
            Event::OpResult(Err(e)) => {
                self.loading = false;
                self.running = None;
                self.error = Some(e);
                Vec::new()
            }
        }
    }

    fn on_key(&mut self, key: KeyKind) -> Vec<Effect> {
        if key == KeyKind::Quit {
            return vec![Effect::Quit];
        }
        match self.screen {
            Screen::Launcher => self.on_key_launcher(key),
            Screen::Main => self.on_key_main(key),
            Screen::PartyOverlay => self.on_key_party(key),
            Screen::Detail => self.on_key_detail(key),
        }
    }

    fn on_key_launcher(&mut self, key: KeyKind) -> Vec<Effect> {
        let n = self.config.profiles.len();
        match key {
            KeyKind::Up if n > 0 => {
                self.selected_profile = (self.selected_profile + n - 1) % n;
            }
            KeyKind::Down if n > 0 => {
                self.selected_profile = (self.selected_profile + 1) % n;
            }
            KeyKind::Enter if n > 0 => {
                self.active_profile = Some(self.selected_profile);
                self.error = None;
                self.status = "Logging in…".to_string();
                return vec![Effect::Login(self.selected_profile)];
            }
            _ => {}
        }
        Vec::new()
    }

    fn on_key_main(&mut self, key: KeyKind) -> Vec<Effect> {
        // Keys that work regardless of which pane is focused.
        match key {
            KeyKind::Tab => {
                self.focus = match self.focus {
                    Focus::Queries => Focus::Results,
                    Focus::Results => Focus::Queries,
                };
                return Vec::new();
            }
            KeyKind::OpenParties => {
                self.screen = Screen::PartyOverlay;
                return Vec::new();
            }
            KeyKind::OpenProfiles => {
                self.screen = Screen::Launcher;
                return Vec::new();
            }
            KeyKind::Refresh => return self.run_selected_op(),
            _ => {}
        }
        match self.focus {
            Focus::Queries => self.on_key_queries(key),
            Focus::Results => self.on_key_results(key),
        }
    }

    fn on_key_queries(&mut self, key: KeyKind) -> Vec<Effect> {
        let n = self.operations.len();
        match key {
            KeyKind::Up if n > 0 => {
                self.selected_op = (self.selected_op + n - 1) % n;
                self.show_cached_for_selected();
            }
            KeyKind::Down if n > 0 => {
                self.selected_op = (self.selected_op + 1) % n;
                self.show_cached_for_selected();
            }
            KeyKind::Enter if n > 0 => return self.run_selected_op(),
            _ => {}
        }
        Vec::new()
    }

    fn on_key_results(&mut self, key: KeyKind) -> Vec<Effect> {
        let n = self.rows_len();
        match key {
            KeyKind::Up if n > 0 => self.result_selected = (self.result_selected + n - 1) % n,
            KeyKind::Down if n > 0 => self.result_selected = (self.result_selected + 1) % n,
            KeyKind::Enter if n > 0 => self.open_detail(),
            KeyKind::Esc => self.focus = Focus::Queries,
            _ => {}
        }
        Vec::new()
    }

    fn on_key_detail(&mut self, key: KeyKind) -> Vec<Effect> {
        let max = self.detail_lines.len().saturating_sub(1);
        match key {
            KeyKind::Up => self.detail_scroll = self.detail_scroll.saturating_sub(1),
            KeyKind::Down => self.detail_scroll = (self.detail_scroll + 1).min(max),
            KeyKind::PageUp => self.detail_scroll = self.detail_scroll.saturating_sub(10),
            KeyKind::PageDown => self.detail_scroll = (self.detail_scroll + 10).min(max),
            KeyKind::Esc => self.screen = Screen::Main,
            _ => {}
        }
        Vec::new()
    }

    fn on_key_party(&mut self, key: KeyKind) -> Vec<Effect> {
        let n = self.parties.len();
        match key {
            KeyKind::Up if n > 0 => self.selected_party = (self.selected_party + n - 1) % n,
            KeyKind::Down if n > 0 => self.selected_party = (self.selected_party + 1) % n,
            KeyKind::Enter if n > 0 => {
                self.active_party = Some(self.parties[self.selected_party].party.clone());
                self.screen = Screen::Main;
                self.focus = Focus::Queries;
                self.show_cached_for_selected();
            }
            KeyKind::Refresh => return vec![Effect::FetchParties],
            KeyKind::Esc => self.screen = Screen::Main,
            _ => {}
        }
        Vec::new()
    }

    /// Mark the selected op as running and request it. Used by Enter and refresh.
    fn run_selected_op(&mut self) -> Vec<Effect> {
        if self.operations.is_empty() {
            return Vec::new();
        }
        let op = self.operations[self.selected_op];
        self.loading = true;
        self.error = None;
        self.status = format!("Running {op}…");
        if let Some(party) = self.active_party.clone() {
            self.running = Some((party, op));
        }
        vec![Effect::RunOp(op)]
    }

    /// Number of rows in the currently displayed table result (0 otherwise).
    fn rows_len(&self) -> usize {
        match &self.result {
            Some(OpResult::Table { rows, .. }) => rows.len(),
            _ => 0,
        }
    }

    /// Show the cached result for the active party + selected op, or clear it.
    fn show_cached_for_selected(&mut self) {
        self.result_selected = 0;
        if self.operations.is_empty() {
            return;
        }
        let key = self
            .active_party
            .clone()
            .map(|p| (p, self.operations[self.selected_op]));
        match key.and_then(|k| self.cache.get(&k)) {
            Some(c) => {
                self.result = Some(c.result.clone());
                self.result_at = Some(c.at);
            }
            None => {
                self.result = None;
                self.result_at = None;
            }
        }
    }

    /// Open the detail view for the selected results row, if it has a payload.
    fn open_detail(&mut self) {
        let detail = match &self.result {
            Some(OpResult::Table { rows, .. }) => {
                rows.get(self.result_selected).and_then(|r| r.detail.clone())
            }
            _ => None,
        };
        if let Some(text) = detail {
            self.detail_lines = text.lines().map(str::to_string).collect();
            self.detail_scroll = 0;
            self.screen = Screen::Detail;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Profile};
    use crate::ops::{Operation, OpResult};
    use crate::session::PartyRight;

    fn app_with_one_profile() -> App {
        let cfg = Config {
            default_profile: None,
            environments: Default::default(),
            profiles: vec![Profile { name: "p1".into(), environment: "devnet".into(), ..Default::default() }],
        };
        App::new(cfg)
    }

    #[test]
    fn enter_on_launcher_requests_login() {
        // Arrange
        let mut app = app_with_one_profile();
        assert_eq!(app.screen, Screen::Launcher);
        // Act
        let effects = app.update(Event::Key(KeyKind::Enter));
        // Assert
        assert_eq!(effects, vec![Effect::Login(0)]);
    }

    #[test]
    fn login_result_ok_enters_main_and_stores_parties() {
        // Arrange
        let mut app = app_with_one_profile();
        app.update(Event::Key(KeyKind::Enter));
        let parties = vec![PartyRight { party: "alice::1220".into(), can_act_as: true, can_read_as: true }];
        // Act
        let effects = app.update(Event::LoginResult(Ok(("token123".into(), parties))));
        // Assert
        assert_eq!(app.screen, Screen::Main);
        assert_eq!(app.active_party.as_deref(), Some("alice::1220"));
        assert!(effects.is_empty());
    }

    #[test]
    fn down_then_enter_runs_selected_op() {
        // Arrange
        let mut app = app_with_one_profile();
        app.update(Event::Key(KeyKind::Enter));
        app.update(Event::LoginResult(Ok((
            "t".into(),
            vec![PartyRight { party: "alice::1220".into(), can_act_as: true, can_read_as: true }],
        ))));
        // Act
        app.update(Event::Key(KeyKind::Down));
        let effects = app.update(Event::Key(KeyKind::Enter));
        // Assert
        assert_eq!(app.selected_op, 1);
        assert!(app.loading);
        assert_eq!(effects, vec![Effect::RunOp(Operation::IncomingOffers)]);
    }

    #[test]
    fn op_result_clears_loading_and_sets_result() {
        // Arrange
        let mut app = app_with_one_profile();
        app.loading = true;
        // Act
        app.update(Event::OpResult(Ok(OpResult::Text {
            title: "t".into(),
            body: "b".into(),
        })));
        // Assert
        assert!(!app.loading);
        assert!(app.result.is_some());
        assert!(app.error.is_none());
    }

    #[test]
    fn op_result_err_sets_error() {
        // Arrange
        let mut app = app_with_one_profile();
        app.loading = true;
        // Act
        app.update(Event::OpResult(Err("boom".into())));
        // Assert
        assert!(!app.loading);
        assert_eq!(app.error.as_deref(), Some("boom"));
    }

    #[test]
    fn enter_on_launcher_sets_logging_in_status() {
        // Arrange
        let mut app = app_with_one_profile();
        // Act
        let effects = app.update(Event::Key(KeyKind::Enter));
        // Assert
        assert_eq!(effects, vec![Effect::Login(0)]);
        assert_eq!(app.status, "Logging in…");
        assert!(app.error.is_none());
    }

    #[test]
    fn login_error_sets_error_and_stays_on_launcher() {
        // Arrange
        let mut app = app_with_one_profile();
        app.update(Event::Key(KeyKind::Enter));
        // Act
        app.update(Event::LoginResult(Err("Invalid user credentials".into())));
        // Assert
        assert_eq!(app.screen, Screen::Launcher);
        assert_eq!(app.error.as_deref(), Some("Invalid user credentials"));
        assert_eq!(app.status, "Login failed");
    }

    #[test]
    fn enter_on_main_sets_running_status() {
        // Arrange: log in to reach the Main screen.
        let mut app = app_with_one_profile();
        app.update(Event::Key(KeyKind::Enter));
        app.update(Event::LoginResult(Ok((
            "t".into(),
            vec![PartyRight { party: "alice::1220".into(), can_act_as: true, can_read_as: true }],
        ))));
        // Act: run the first (default-selected) operation.
        app.update(Event::Key(KeyKind::Enter));
        // Assert
        assert!(app.loading);
        assert_eq!(app.status, "Running Check Balance…");
    }

    #[test]
    fn relogin_preserves_active_party() {
        // Arrange: log in, then switch to the second party.
        let mut app = app_with_one_profile();
        let parties = vec![
            PartyRight { party: "first::1220".into(), can_act_as: true, can_read_as: true },
            PartyRight { party: "funded::1220".into(), can_act_as: true, can_read_as: true },
        ];
        app.update(Event::Key(KeyKind::Enter));
        app.update(Event::LoginResult(Ok(("t1".into(), parties.clone()))));
        app.update(Event::Key(KeyKind::OpenParties));
        app.update(Event::Key(KeyKind::Down));
        app.update(Event::Key(KeyKind::Enter));
        assert_eq!(app.active_party.as_deref(), Some("funded::1220"));
        // Act: token expires; user re-logs in (same party list).
        app.update(Event::LoginResult(Ok(("t2".into(), parties.clone()))));
        // Assert: still on the chosen party, not reset to the first.
        assert_eq!(app.active_party.as_deref(), Some("funded::1220"));
        assert_eq!(app.selected_party, 1);
    }

    fn sample_table() -> OpResult {
        OpResult::Table {
            title: "t".into(),
            columns: vec!["c".into()],
            rows: vec![crate::ops::ResultRow::new(
                vec!["v".into()],
                Some("line1\nline2\nline3".into()),
            )],
        }
    }

    fn logged_in() -> App {
        let mut app = app_with_one_profile();
        app.update(Event::Key(KeyKind::Enter));
        app.update(Event::LoginResult(Ok((
            "t".into(),
            vec![PartyRight { party: "alice::1220".into(), can_act_as: true, can_read_as: true }],
        ))));
        app
    }

    #[test]
    fn tab_toggles_focus() {
        let mut app = logged_in();
        assert_eq!(app.focus, Focus::Queries);
        app.update(Event::Key(KeyKind::Tab));
        assert_eq!(app.focus, Focus::Results);
        app.update(Event::Key(KeyKind::Tab));
        assert_eq!(app.focus, Focus::Queries);
    }

    #[test]
    fn navigating_ops_shows_and_clears_cached_result() {
        let mut app = logged_in();
        // Run op 0 and cache its result.
        app.update(Event::Key(KeyKind::Enter));
        app.update(Event::OpResult(Ok(sample_table())));
        assert!(app.result.is_some());
        // Move to op 1 — no cache yet.
        app.update(Event::Key(KeyKind::Down));
        assert!(app.result.is_none());
        // Back to op 0 — served from cache, no re-query.
        app.update(Event::Key(KeyKind::Up));
        assert!(app.result.is_some());
        assert!(app.result_at.is_some());
    }

    #[test]
    fn results_focus_enter_opens_detail_then_esc_returns() {
        let mut app = logged_in();
        app.update(Event::Key(KeyKind::Enter));
        app.update(Event::OpResult(Ok(sample_table())));
        // Focus the results pane, open the selected row's detail.
        app.update(Event::Key(KeyKind::Tab));
        assert_eq!(app.focus, Focus::Results);
        app.update(Event::Key(KeyKind::Enter));
        assert_eq!(app.screen, Screen::Detail);
        assert_eq!(app.detail_lines.len(), 3);
        // Scroll, then escape back to main.
        app.update(Event::Key(KeyKind::Down));
        assert_eq!(app.detail_scroll, 1);
        app.update(Event::Key(KeyKind::Esc));
        assert_eq!(app.screen, Screen::Main);
    }
}
