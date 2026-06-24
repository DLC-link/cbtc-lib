use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
};

use crate::app::{App, Focus, FormKind, Screen};
use crate::ops::OpResult;
use crate::theme::{Role, Theme, glyph};

/// Render the whole UI for the current frame.
pub fn draw(frame: &mut Frame, app: &App, theme: &Theme, spinner_frame: usize) {
    match app.screen {
        Screen::Launcher => draw_launcher(frame, app, theme),
        Screen::Main => draw_main(frame, app, theme, spinner_frame),
        Screen::PartyOverlay => {
            draw_main(frame, app, theme, spinner_frame);
            draw_party_overlay(frame, app, theme);
        }
        Screen::Detail => {
            draw_main(frame, app, theme, spinner_frame);
            draw_detail(frame, app, theme);
        }
        Screen::ActionMenu => {
            draw_main(frame, app, theme, spinner_frame);
            draw_action_menu(frame, app, theme);
        }
        Screen::Form => {
            draw_main(frame, app, theme, spinner_frame);
            draw_form(frame, app, theme);
        }
        Screen::Confirm => {
            draw_main(frame, app, theme, spinner_frame);
            draw_confirm(frame, app, theme);
        }
    }
}

fn draw_launcher(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = frame.area();
    let default = app.config.default_profile.as_deref();

    let header = Row::new(["", "PROFILE", "ENV", "USER"].map(Cell::from)).style(
        Style::default()
            .fg(theme.color(Role::FgDim))
            .add_modifier(Modifier::BOLD),
    );
    let rows = app.config.profiles.iter().map(|p| {
        // Mark the default profile (the one that auto-logs-in on launch) with the
        // brand diamond, aligned in its own leading column.
        let marker = if default == Some(p.name.as_str()) {
            Cell::from(glyph::DIAMOND).style(Style::default().fg(theme.color(Role::Accent)))
        } else {
            Cell::from("")
        };
        Row::new(vec![
            marker,
            Cell::from(p.name.clone()),
            Cell::from(p.environment.clone()),
            Cell::from(p.keycloak_username.clone()),
        ])
    });
    // Fixed marker/env columns; name and user share the remaining width so the
    // columns stay aligned regardless of how long any single value is.
    let widths = [
        Constraint::Length(1),
        Constraint::Min(14),
        Constraint::Length(10),
        Constraint::Min(14),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(2)
        .block(
            Block::default()
                .title(Span::styled(
                    "cbtc-tui · PROFILES",
                    Style::default()
                        .fg(theme.color(Role::Accent))
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.color(Role::FgDim))),
        )
        .row_highlight_style(
            Style::default()
                .fg(theme.color(Role::Accent))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut state = TableState::default();
    if !app.config.profiles.is_empty() {
        state.select(Some(
            app.selected_profile.min(app.config.profiles.len() - 1),
        ));
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1), Constraint::Length(1)])
        .split(area);
    frame.render_stateful_widget(table, chunks[0], &mut state);
    // Status / error line: login progress, or the last error in red.
    let (msg, role) = match &app.error {
        Some(err) => (format!("{} {err}", glyph::CROSS), Role::Danger),
        None => (app.status.clone(), Role::FgDim),
    };
    frame.render_widget(
        Paragraph::new(msg).style(Style::default().fg(theme.color(role))),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "↑/↓ select · Enter activate · {} default · q quit",
            glyph::DIAMOND
        ))
        .style(Style::default().fg(theme.color(Role::FgDim))),
        chunks[2],
    );
}

fn draw_main(frame: &mut Frame, app: &App, theme: &Theme, spinner_frame: usize) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    // Status bar.
    let profile_name = app
        .active_profile
        .and_then(|i| app.config.profiles.get(i))
        .map(|p| p.name.clone())
        .unwrap_or_default();
    let party = app.active_party.clone().unwrap_or_else(|| "<none>".to_string());
    frame.render_widget(
        Paragraph::new(format!("{} Profile: {profile_name}   Party: {party}", glyph::DIAMOND))
            .style(Style::default().fg(theme.color(Role::Accent))),
        rows[0],
    );

    // Body: op list (left) + results (right).
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Min(20)])
        .split(rows[1]);

    let items: Vec<ListItem> = app
        .operations
        .iter()
        .enumerate()
        .map(|(i, op)| {
            let style = if i == app.selected_op {
                Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(op.to_string()).style(style)
        })
        .collect();
    let queries_border = if app.focus == Focus::Queries {
        Role::Accent
    } else {
        Role::FgDim
    };
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .title(Span::styled(
                    "QUERIES",
                    Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.color(queries_border))),
        ),
        body[0],
    );

    draw_results(frame, app, theme, spinner_frame, body[1], app.focus == Focus::Results);

    // Footer.
    frame.render_widget(
        Paragraph::new(
            "Tab pane · ↑↓ · Enter run/detail · a actions · p party · P prof · q quit",
        )
        .style(Style::default().fg(theme.color(Role::FgDim))),
        rows[2],
    );
}

fn draw_results(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    spinner_frame: usize,
    area: Rect,
    focused: bool,
) {
    let border_role = if focused { Role::Accent } else { Role::FgDim };
    let title = if app.loading {
        "Working…".to_string()
    } else if app.error.is_some() {
        "Error".to_string()
    } else {
        result_title(app)
    };
    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.color(border_role)));

    if app.loading {
        let frames = glyph::SPINNER;
        let s = frames[spinner_frame % frames.len()];
        let label = if app.status.is_empty() { "Running…" } else { app.status.as_str() };
        frame.render_widget(
            Paragraph::new(format!("{s} {label}"))
                .style(Style::default().fg(theme.color(Role::Accent)))
                .block(block),
            area,
        );
        return;
    }
    if let Some(err) = &app.error {
        frame.render_widget(
            Paragraph::new(Line::from(format!("{} {err}", glyph::CROSS)))
                .style(Style::default().fg(theme.color(Role::Danger)))
                .block(block),
            area,
        );
        return;
    }
    match &app.result {
        Some(OpResult::Table { columns, rows, .. }) => {
            let header = Row::new(columns.iter().map(|c| Cell::from(c.clone())))
                .style(Style::default().fg(theme.color(Role::FgDim)));
            let widths: Vec<Constraint> = columns
                .iter()
                .map(|_| Constraint::Percentage(100 / columns.len().max(1) as u16))
                .collect();
            let mut table = Table::new(
                rows.iter().map(|r| {
                    let row = Row::new(r.cells.iter().map(|c| Cell::from(c.clone())));
                    if r.expired {
                        row.style(Style::default().fg(theme.color(Role::FgDim)))
                    } else {
                        row
                    }
                }),
                widths,
            )
            .header(header)
            .block(block);
            let mut state = TableState::default();
            if focused {
                table = table
                    .row_highlight_style(
                        Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");
                if !rows.is_empty() {
                    state.select(Some(app.result_selected.min(rows.len() - 1)));
                }
            }
            frame.render_stateful_widget(table, area, &mut state);
        }
        Some(OpResult::Text { body, .. }) => {
            frame.render_widget(Paragraph::new(body.clone()).block(block), area);
        }
        None => {
            frame.render_widget(
                Paragraph::new("Select a query and press Enter to run.").block(block),
                area,
            );
        }
    }
}

/// Title for the results pane: the result's own title plus a cache-age suffix.
fn result_title(app: &App) -> String {
    let base = match &app.result {
        Some(OpResult::Table { title, .. }) | Some(OpResult::Text { title, .. }) => title.clone(),
        None => app
            .operations
            .get(app.selected_op)
            .map(|o| o.to_string())
            .unwrap_or_else(|| "Results".to_string()),
    };
    match app.result_at {
        Some(at) => format!("{base} — {}", age(at)),
        None => base,
    }
}

/// Human-readable age of the currently displayed (possibly cached) result.
fn age(at: Instant) -> String {
    let secs = at.elapsed().as_secs();
    if secs < 5 {
        "just now".to_string()
    } else if secs < 60 {
        format!("cached {secs}s ago")
    } else if secs < 3600 {
        format!("cached {}m ago", secs / 60)
    } else {
        format!("cached {}h ago", secs / 3600)
    }
}

fn draw_detail(frame: &mut Frame, app: &App, theme: &Theme) {
    // Centered popup over the (still-visible) main screen.
    let area = centered_rect(70, 70, frame.area());
    frame.render_widget(Clear, area);
    let text = app.detail_lines.join("\n");
    let para = Paragraph::new(text)
        .block(
            Block::default()
                .title(Span::styled(
                    "Detail · ↑↓ PgUp/PgDn scroll · Esc close",
                    Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.color(Role::Accent))),
        )
        .scroll((app.detail_scroll as u16, 0));
    frame.render_widget(para, area);
}

fn draw_action_menu(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = centered_rect(40, 30, frame.area());
    frame.render_widget(Clear, area);
    let items: Vec<ListItem> = app
        .action_items
        .iter()
        .map(|(label, _)| ListItem::new(label.clone()))
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    "Actions · Enter · Esc",
                    Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.color(Role::Accent))),
        )
        .highlight_style(
            Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut state = ListState::default();
    if !app.action_items.is_empty() {
        state.select(Some(app.action_selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_form(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = centered_rect(60, 30, frame.area());
    frame.render_widget(Clear, area);
    let title = match app.form.as_ref().map(|f| &f.kind) {
        Some(FormKind::CreateWithdrawAccount) => "New withdraw account",
        Some(FormKind::SubmitWithdraw { .. }) => "Submit withdraw",
        None => "Input",
    };
    let (label, input, error) = match &app.form {
        Some(f) => (f.label.as_str(), f.input.as_str(), f.error.as_deref()),
        None => ("", "", None),
    };
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(label.to_string(), Style::default().fg(theme.color(Role::FgDim)))),
        Line::from(format!("> {input}█")),
        Line::from(""),
    ];
    if let Some(e) = error {
        lines.push(Line::from(Span::styled(
            format!("{} {e}", glyph::CROSS),
            Style::default().fg(theme.color(Role::Danger)),
        )));
    }
    lines.push(Line::from(Span::styled(
        "Enter submit · Esc cancel",
        Style::default().fg(theme.color(Role::FgDim)),
    )));
    let para = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(
                title,
                Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color(Role::Accent))),
    );
    frame.render_widget(para, area);
}

fn draw_confirm(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = centered_rect(60, 40, frame.area());
    frame.render_widget(Clear, area);
    let mainnet = app.is_mainnet();
    let summary = app.pending.as_ref().map(|(_, s)| s.clone()).unwrap_or_default();
    let mut lines: Vec<Line> = Vec::new();
    if mainnet {
        lines.push(Line::from(Span::styled(
            "⚠ MAINNET ⚠",
            Style::default().fg(theme.color(Role::Danger)).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }
    for l in summary.lines() {
        lines.push(Line::from(l.to_string()));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        Span::styled("Enter confirm · Esc cancel", Style::default().fg(theme.color(Role::FgDim))),
    ));
    let border = if mainnet { Role::Danger } else { Role::Accent };
    let para = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(
                "Confirm",
                Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color(border))),
    );
    frame.render_widget(para, area);
}

fn draw_party_overlay(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = centered_rect(60, 50, frame.area());
    let items: Vec<ListItem> = app
        .parties
        .iter()
        .map(|p| {
            let rights = match (p.can_act_as, p.can_read_as) {
                (true, true) => "[act + read]",
                (true, false) => "[act]",
                (false, true) => "[read]",
                _ => "[none]",
            };
            ListItem::new(format!("{}   {rights}", p.party))
        })
        .collect();
    let count = app.parties.len();
    frame.render_widget(Clear, area);
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    "Switch party ({}/{count}) · Enter · r refresh · Esc",
                    if count == 0 { 0 } else { app.selected_party + 1 }
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.color(Role::Accent))),
        )
        .highlight_style(
            Style::default()
                .fg(theme.color(Role::Accent))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    // A fresh ListState each frame, selecting the active row, lets ratatui
    // auto-scroll the viewport so the cursor stays visible with long lists.
    let mut state = ListState::default();
    if count > 0 {
        state.select(Some(app.selected_party));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(v[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Screen};
    use crate::config::{Config, Profile};
    use crate::theme::Theme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn main_screen_renders_op_list_and_footer() {
        // Arrange
        let cfg = Config {
            default_profile: None,
            environments: Default::default(),
            profiles: vec![Profile {
                name: "p1".into(),
                environment: "devnet".into(),
                ..Default::default()
            }],
        };
        let mut app = App::new(cfg);
        app.screen = Screen::Main;
        app.active_party = Some("alice::1220abcd".into());
        let theme = Theme { truecolor: true };
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        // Act
        terminal.draw(|f| draw(f, &app, &theme, 0)).unwrap();
        // Assert
        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("Check Balance"));
        assert!(text.contains("quit"));
    }

    #[test]
    fn launcher_renders_login_error() {
        // Arrange
        let cfg = Config {
            default_profile: None,
            environments: Default::default(),
            profiles: vec![Profile {
                name: "p1".into(),
                environment: "devnet".into(),
                ..Default::default()
            }],
        };
        let mut app = App::new(cfg);
        app.screen = Screen::Launcher;
        app.error = Some("Invalid user credentials".into());
        let theme = Theme { truecolor: true };
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        // Act
        terminal.draw(|f| draw(f, &app, &theme, 0)).unwrap();
        // Assert
        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("Invalid user credentials"));
    }

    #[test]
    fn launcher_renders_profile_table_with_default_marker() {
        // Arrange: two profiles of differing name length, mainnet is the default.
        let cfg = Config {
            default_profile: Some("mainnet".into()),
            environments: Default::default(),
            profiles: vec![
                Profile {
                    name: "devnet".into(),
                    environment: "devnet".into(),
                    keycloak_username: "merchant_user".into(),
                    ..Default::default()
                },
                Profile {
                    name: "mainnet".into(),
                    environment: "mainnet".into(),
                    keycloak_username: "cbtc-incentive-sender".into(),
                    ..Default::default()
                },
            ],
        };
        let mut app = App::new(cfg);
        app.screen = Screen::Launcher;
        let theme = Theme { truecolor: true };
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        // Act
        terminal.draw(|f| draw(f, &app, &theme, 0)).unwrap();
        // Assert: column headers, both profiles, and the default-marker glyph.
        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("PROFILE"));
        assert!(text.contains("ENV"));
        assert!(text.contains("USER"));
        assert!(text.contains("devnet"));
        assert!(text.contains("mainnet"));
        assert!(text.contains(glyph::DIAMOND));
    }

    #[test]
    fn party_overlay_scrolls_to_selected() {
        use crate::session::PartyRight;
        // Arrange: many parties, cursor near the bottom.
        let cfg = Config {
            default_profile: None,
            environments: Default::default(),
            profiles: vec![Profile {
                name: "p1".into(),
                environment: "devnet".into(),
                ..Default::default()
            }],
        };
        let mut app = App::new(cfg);
        app.screen = Screen::PartyOverlay;
        app.parties = (0..40)
            .map(|i| PartyRight {
                party: format!("party-{i:02}::1220"),
                can_act_as: true,
                can_read_as: true,
            })
            .collect();
        app.selected_party = 37;
        let theme = Theme { truecolor: true };
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        // Act
        terminal.draw(|f| draw(f, &app, &theme, 0)).unwrap();
        // Assert: the selected row scrolled into view; the top row is off-screen.
        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("party-37"), "selected party should be visible");
        assert!(!text.contains("party-00"), "top party should have scrolled off");
    }

    #[test]
    fn detail_screen_renders_payload() {
        // Arrange
        let mut app = App::new(Config::default());
        app.screen = Screen::Detail;
        app.detail_lines = vec!["\"owner\": \"alice::1220ab\"".into(), "\"id\": \"acct-7\"".into()];
        let theme = Theme { truecolor: true };
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        // Act
        terminal.draw(|f| draw(f, &app, &theme, 0)).unwrap();
        // Assert
        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("acct-7"));
        assert!(text.contains("Detail"));
    }

    #[test]
    fn action_menu_popup_renders() {
        let mut app = App::new(Config::default());
        app.screen = Screen::ActionMenu;
        app.action_items = vec![
            (
                "Accept".into(),
                crate::app::PendingAction::Command(crate::app::Command::Accept { cid: "x".into() }),
            ),
            (
                "Reject".into(),
                crate::app::PendingAction::Command(crate::app::Command::Reject { cid: "x".into() }),
            ),
        ];
        let theme = Theme { truecolor: true };
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| draw(f, &app, &theme, 0)).unwrap();
        let text: String = terminal.backend().buffer().clone().content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("Actions"));
        assert!(text.contains("Accept"));
        assert!(text.contains("Reject"));
    }

    #[test]
    fn confirm_popup_renders_with_mainnet_banner() {
        let cfg = Config {
            default_profile: None,
            environments: Default::default(),
            profiles: vec![Profile {
                name: "p".into(),
                environment: "mainnet".into(),
                ..Default::default()
            }],
        };
        let mut app = App::new(cfg);
        app.active_profile = Some(0);
        app.screen = Screen::Confirm;
        app.pending = Some((
            crate::app::Command::Accept { cid: "00cid".into() },
            "Accept:\nbob::1220  0.5".into(),
        ));
        let theme = Theme { truecolor: true };
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| draw(f, &app, &theme, 0)).unwrap();
        let text: String = terminal.backend().buffer().clone().content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("Confirm"));
        assert!(text.contains("MAINNET"));
        assert!(text.contains("Accept"));
    }

    #[test]
    fn form_popup_renders() {
        let mut app = App::new(Config::default());
        app.screen = Screen::Form;
        app.form = Some(crate::app::FormState {
            kind: crate::app::FormKind::CreateWithdrawAccount,
            label: "Destination BTC address".into(),
            input: "bc1qexample".into(),
            error: None,
        });
        let theme = Theme { truecolor: true };
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| draw(f, &app, &theme, 0)).unwrap();
        let text: String = terminal.backend().buffer().clone().content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("New withdraw account"));
        assert!(text.contains("bc1qexample"));
    }
}
