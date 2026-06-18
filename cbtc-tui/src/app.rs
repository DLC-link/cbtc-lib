use strum::IntoEnumIterator;

use crate::config::Config;
use crate::ops::{Operation, OpResult};
use crate::session::PartyRight;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Launcher,
    Main,
    PartyOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    Up,
    Down,
    Enter,
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

/// Mutable UI state plus the pure transition function.
pub struct App {
    pub config: Config,
    pub screen: Screen,
    pub selected_profile: usize,
    pub active_profile: Option<usize>,
    pub access_token: Option<String>,
    pub parties: Vec<PartyRight>,
    pub selected_party: usize,
    pub active_party: Option<String>,
    pub operations: Vec<Operation>,
    pub selected_op: usize,
    pub result: Option<OpResult>,
    pub loading: bool,
    pub error: Option<String>,
    pub status: String,
}

impl App {
    pub fn new(config: Config) -> Self {
        App {
            config,
            screen: Screen::Launcher,
            selected_profile: 0,
            active_profile: None,
            access_token: None,
            parties: Vec::new(),
            selected_party: 0,
            active_party: None,
            operations: Operation::iter().collect(),
            selected_op: 0,
            result: None,
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
                self.active_party = parties.first().map(|p| p.party.clone());
                self.selected_party = 0;
                self.parties = parties;
                self.screen = Screen::Main;
                self.error = None;
                self.status = "Logged in".to_string();
                Vec::new()
            }
            Event::LoginResult(Err(e)) => {
                self.error = Some(e);
                self.status = "Login failed".to_string();
                Vec::new()
            }
            Event::OpResult(Ok(result)) => {
                self.loading = false;
                self.result = Some(result);
                self.error = None;
                Vec::new()
            }
            Event::OpResult(Err(e)) => {
                self.loading = false;
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
        let n = self.operations.len();
        match key {
            KeyKind::Up if n > 0 => self.selected_op = (self.selected_op + n - 1) % n,
            KeyKind::Down if n > 0 => self.selected_op = (self.selected_op + 1) % n,
            KeyKind::Enter if n > 0 => {
                self.loading = true;
                self.error = None;
                self.status = format!("Running {}…", self.operations[self.selected_op]);
                return vec![Effect::RunOp(self.operations[self.selected_op])];
            }
            KeyKind::OpenParties => self.screen = Screen::PartyOverlay,
            KeyKind::OpenProfiles => self.screen = Screen::Launcher,
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
            }
            KeyKind::Refresh => return vec![Effect::FetchParties],
            KeyKind::Esc => self.screen = Screen::Main,
            _ => {}
        }
        Vec::new()
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
}
