use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
};

use crate::app::{App, Focus, Screen};
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
    }
}

fn draw_launcher(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = frame.area();
    let items: Vec<ListItem> = app
        .config
        .profiles
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let label = format!("{}   {}   {}", p.name, p.environment, p.keycloak_username);
            let style = if i == app.selected_profile {
                Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(label).style(style)
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .title("cbtc-tui · PROFILES")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color(Role::FgDim))),
    );
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1), Constraint::Length(1)])
        .split(area);
    frame.render_widget(list, chunks[0]);
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
        Paragraph::new("↑/↓ select · Enter activate · q quit")
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
            "Tab pane · ↑↓ · Enter run/detail · p party · P profiles · r refresh · q quit",
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
                rows.iter().map(|r| Row::new(r.cells.iter().map(|c| Cell::from(c.clone())))),
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
}
