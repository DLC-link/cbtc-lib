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
    ActionMenu,
    Confirm,
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
    Action,
    Quit,
    Esc,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Key(KeyKind),
    LoginResult(std::result::Result<(String, Vec<PartyRight>), String>),
    OpResult(std::result::Result<OpResult, String>),
    CommandResult(std::result::Result<String, String>),
}

/// A write command targeting a specific contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Accept { cid: String },
    Reject { cid: String },
    Cancel { cid: String },
    /// Batch-cancel a set of (expired) outgoing offers.
    CancelExpired { cids: Vec<String> },
    /// Merge (consolidate) all of the party's CBTC holdings into one.
    MergeHoldings,
}

impl Command {
    /// The (primary) contract id this command targets, if any.
    pub fn cid(&self) -> &str {
        match self {
            Command::Accept { cid } | Command::Reject { cid } | Command::Cancel { cid } => cid,
            Command::CancelExpired { cids } => cids.first().map(String::as_str).unwrap_or(""),
            Command::MergeHoldings => "",
        }
    }

    /// Short verb for labels and status messages.
    pub fn verb(&self) -> &'static str {
        match self {
            Command::Accept { .. } => "Accept",
            Command::Reject { .. } => "Reject",
            Command::Cancel { .. } => "Cancel",
            Command::CancelExpired { .. } => "Cancel expired",
            Command::MergeHoldings => "Merge holdings",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Quit,
    Login(usize),
    RunOp(Operation),
    RunCommand(Command),
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
    /// Context actions (label, command) shown in the actions menu.
    pub action_items: Vec<(String, Command)>,
    pub action_selected: usize,
    /// The command awaiting confirmation, with a human-readable summary.
    pub pending: Option<(Command, String)>,
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
            action_items: Vec::new(),
            action_selected: 0,
            pending: None,
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
            Event::CommandResult(Ok(msg)) => {
                self.status = msg;
                self.error = None;
                // Invalidate this party+op cache entry and refresh so the list
                // reflects the command's effect.
                if let Some(party) = self.active_party.clone() {
                    let op = self.operations[self.selected_op];
                    self.cache.remove(&(party.clone(), op));
                    self.running = Some((party, op));
                    self.loading = true;
                    return vec![Effect::RunOp(op)];
                }
                Vec::new()
            }
            Event::CommandResult(Err(e)) => {
                self.loading = false;
                self.error = Some(format!("command failed: {e}"));
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
            Screen::ActionMenu => self.on_key_action_menu(key),
            Screen::Confirm => self.on_key_confirm(key),
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
            KeyKind::Action if n > 0 => self.open_action_menu(),
            KeyKind::Esc => self.focus = Focus::Queries,
            _ => {}
        }
        Vec::new()
    }

    fn on_key_action_menu(&mut self, key: KeyKind) -> Vec<Effect> {
        let n = self.action_items.len();
        match key {
            KeyKind::Up if n > 0 => self.action_selected = (self.action_selected + n - 1) % n,
            KeyKind::Down if n > 0 => self.action_selected = (self.action_selected + 1) % n,
            KeyKind::Enter if n > 0 => {
                let (label, command) = self.action_items[self.action_selected].clone();
                let summary = self.command_summary(&label, &command);
                self.pending = Some((command, summary));
                self.screen = Screen::Confirm;
            }
            KeyKind::Esc => self.screen = Screen::Main,
            _ => {}
        }
        Vec::new()
    }

    fn on_key_confirm(&mut self, key: KeyKind) -> Vec<Effect> {
        match key {
            KeyKind::Enter => {
                if let Some((command, _)) = self.pending.take() {
                    self.screen = Screen::Main;
                    self.loading = true;
                    self.error = None;
                    self.status = format!("Submitting {}…", command.verb());
                    return vec![Effect::RunCommand(command)];
                }
                self.screen = Screen::Main;
            }
            KeyKind::Esc => {
                self.pending = None;
                self.screen = Screen::Main;
            }
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

    /// Build the context-aware actions for the selected results row and open the menu.
    fn open_action_menu(&mut self) {
        let (row_cid, expired_cids): (Option<String>, Vec<String>) = match &self.result {
            Some(OpResult::Table { rows, .. }) => {
                let row_cid = rows.get(self.result_selected).and_then(|r| r.id.clone());
                let expired = rows
                    .iter()
                    .filter(|r| r.expired)
                    .filter_map(|r| r.id.clone())
                    .collect();
                (row_cid, expired)
            }
            _ => (None, Vec::new()),
        };
        let mut items: Vec<(String, Command)> = Vec::new();
        match self.operations[self.selected_op] {
            Operation::CheckBalance => {
                let holdings = self.rows_len();
                if holdings >= 2 {
                    items.push((
                        format!("Merge holdings ({holdings})"),
                        Command::MergeHoldings,
                    ));
                }
            }
            Operation::IncomingOffers => {
                if let Some(cid) = row_cid {
                    items.push(("Accept".to_string(), Command::Accept { cid: cid.clone() }));
                    items.push(("Reject".to_string(), Command::Reject { cid }));
                }
            }
            Operation::OutgoingOffers => {
                if let Some(cid) = row_cid {
                    items.push(("Cancel".to_string(), Command::Cancel { cid }));
                }
                if !expired_cids.is_empty() {
                    items.push((
                        format!("Cancel all expired ({})", expired_cids.len()),
                        Command::CancelExpired { cids: expired_cids },
                    ));
                }
            }
            _ => {}
        }
        if items.is_empty() {
            return;
        }
        self.action_items = items;
        self.action_selected = 0;
        self.screen = Screen::ActionMenu;
    }

    /// Human-readable summary of a pending command for the confirm popup.
    fn command_summary(&self, label: &str, command: &Command) -> String {
        if let Command::CancelExpired { cids } = command {
            return format!("Cancel {} expired outgoing offer(s)", cids.len());
        }
        if let Command::MergeHoldings = command {
            return format!("Merge {} holdings into one", self.rows_len());
        }
        let detail = match &self.result {
            Some(OpResult::Table { rows, .. }) => rows
                .get(self.result_selected)
                .map(|r| r.cells.join("  "))
                .unwrap_or_default(),
            _ => command.cid().to_string(),
        };
        format!("{label}:\n{detail}")
    }

    /// True when the active profile targets the mainnet environment.
    pub fn is_mainnet(&self) -> bool {
        self.active_profile
            .and_then(|i| self.config.profiles.get(i))
            .map(|p| p.environment == "mainnet")
            .unwrap_or(false)
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

    fn offer_table() -> OpResult {
        OpResult::Table {
            title: "Incoming Offers".into(),
            columns: vec!["From".into(), "Amount".into()],
            rows: vec![
                crate::ops::ResultRow::new(vec!["bob::1220".into(), "0.5".into()], None)
                    .with_id("00offercid".into()),
            ],
        }
    }

    /// Put a logged-in app on Incoming Offers with one selectable offer row, results focused.
    fn on_incoming_offers() -> App {
        let mut app = logged_in();
        app.selected_op = 1; // IncomingOffers (CheckBalance=0)
        assert_eq!(app.operations[app.selected_op], Operation::IncomingOffers);
        app.result = Some(offer_table());
        app.focus = Focus::Results;
        app.result_selected = 0;
        app
    }

    #[test]
    fn accept_offer_flow() {
        let mut app = on_incoming_offers();
        // 'a' opens a context actions menu with Accept/Reject.
        app.update(Event::Key(KeyKind::Action));
        assert_eq!(app.screen, Screen::ActionMenu);
        assert_eq!(app.action_items.len(), 2);
        assert_eq!(app.action_items[0].0, "Accept");
        assert_eq!(app.action_items[1].0, "Reject");
        // Enter → confirmation popup with a pending command.
        app.update(Event::Key(KeyKind::Enter));
        assert_eq!(app.screen, Screen::Confirm);
        assert!(app.pending.is_some());
        // Enter → submit the command.
        let effects = app.update(Event::Key(KeyKind::Enter));
        assert_eq!(effects, vec![Effect::RunCommand(Command::Accept { cid: "00offercid".into() })]);
        assert!(app.loading);
        // Command result → invalidate cache + refresh the offer list.
        let refresh = app.update(Event::CommandResult(Ok("Accepted offer".into())));
        assert_eq!(refresh, vec![Effect::RunOp(Operation::IncomingOffers)]);
        assert_eq!(app.status, "Accepted offer");
    }

    #[test]
    fn action_menu_esc_cancels() {
        let mut app = on_incoming_offers();
        app.update(Event::Key(KeyKind::Action));
        assert_eq!(app.screen, Screen::ActionMenu);
        app.update(Event::Key(KeyKind::Esc));
        assert_eq!(app.screen, Screen::Main);
        assert!(app.pending.is_none());
    }

    #[test]
    fn balance_row_has_no_actions() {
        // On a non-offer screen, 'a' does nothing.
        let mut app = logged_in();
        app.selected_op = 0; // CheckBalance
        app.result = Some(sample_table());
        app.focus = Focus::Results;
        app.update(Event::Key(KeyKind::Action));
        assert_eq!(app.screen, Screen::Main);
    }

    #[test]
    fn cancel_all_expired_flow() {
        let mut app = logged_in();
        app.selected_op = 2; // OutgoingOffers
        assert_eq!(app.operations[app.selected_op], Operation::OutgoingOffers);
        app.result = Some(OpResult::Table {
            title: "Outgoing Offers".into(),
            columns: vec!["To".into()],
            rows: vec![
                crate::ops::ResultRow::new(vec!["a".into()], None)
                    .with_id("00a".into())
                    .with_expired(true),
                crate::ops::ResultRow::new(vec!["b".into()], None)
                    .with_id("00b".into())
                    .with_expired(false),
                crate::ops::ResultRow::new(vec!["c".into()], None)
                    .with_id("00c".into())
                    .with_expired(true),
            ],
        });
        app.focus = Focus::Results;
        app.result_selected = 1; // a non-expired row is selected
        app.update(Event::Key(KeyKind::Action));
        assert_eq!(app.screen, Screen::ActionMenu);
        // Row "Cancel" + global "Cancel all expired (2)".
        assert_eq!(app.action_items.len(), 2);
        assert!(app.action_items[1].0.contains("Cancel all expired (2)"));
        // Pick the global action → confirm → submit.
        app.update(Event::Key(KeyKind::Down));
        app.update(Event::Key(KeyKind::Enter));
        assert_eq!(app.screen, Screen::Confirm);
        let effects = app.update(Event::Key(KeyKind::Enter));
        assert_eq!(
            effects,
            vec![Effect::RunCommand(Command::CancelExpired {
                cids: vec!["00a".into(), "00c".into()]
            })]
        );
    }

    #[test]
    fn merge_holdings_flow() {
        let mut app = logged_in();
        app.selected_op = 0; // CheckBalance
        assert_eq!(app.operations[app.selected_op], Operation::CheckBalance);
        app.result = Some(OpResult::Table {
            title: "Total CBTC: 1".into(),
            columns: vec!["#".into(), "Amount".into()],
            rows: vec![
                crate::ops::ResultRow::new(vec!["1".into(), "0.5".into()], None),
                crate::ops::ResultRow::new(vec!["2".into(), "0.5".into()], None),
            ],
        });
        app.focus = Focus::Results;
        app.update(Event::Key(KeyKind::Action));
        assert_eq!(app.screen, Screen::ActionMenu);
        assert_eq!(app.action_items.len(), 1);
        assert!(app.action_items[0].0.contains("Merge holdings (2)"));
        app.update(Event::Key(KeyKind::Enter)); // → Confirm
        assert_eq!(app.screen, Screen::Confirm);
        let effects = app.update(Event::Key(KeyKind::Enter)); // → submit
        assert_eq!(effects, vec![Effect::RunCommand(Command::MergeHoldings)]);
    }
}
