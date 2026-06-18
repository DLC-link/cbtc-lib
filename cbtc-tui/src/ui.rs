use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table};

use crate::app::{App, Screen};
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
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .title("QUERIES")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.color(Role::FgDim))),
        ),
        body[0],
    );

    draw_results(frame, app, theme, spinner_frame, body[1]);

    // Footer.
    frame.render_widget(
        Paragraph::new("↑/↓ select · Enter run · p party · P profiles · r refresh · q quit")
            .style(Style::default().fg(theme.color(Role::FgDim))),
        rows[2],
    );
}

fn draw_results(frame: &mut Frame, app: &App, theme: &Theme, spinner_frame: usize, area: Rect) {
    let block = Block::default()
        .title("RESULTS")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.color(Role::FgDim)));
    if app.loading {
        let frames = glyph::SPINNER;
        let s = frames[spinner_frame % frames.len()];
        frame.render_widget(Paragraph::new(format!("{s} Running…")).block(block), area);
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
        Some(OpResult::Table { title, columns, rows }) => {
            let header = Row::new(columns.iter().map(|c| Cell::from(c.clone())))
                .style(Style::default().fg(theme.color(Role::FgDim)));
            let widths: Vec<Constraint> = columns
                .iter()
                .map(|_| Constraint::Percentage(100 / columns.len().max(1) as u16))
                .collect();
            let table = Table::new(
                rows.iter().map(|r| Row::new(r.iter().map(|c| Cell::from(c.clone())))),
                widths,
            )
            .header(header)
            .block(block.title(title.clone()));
            frame.render_widget(table, area);
        }
        Some(OpResult::Text { title, body }) => {
            frame.render_widget(
                Paragraph::new(body.clone()).block(block.title(title.clone())),
                area,
            );
        }
        None => {
            frame.render_widget(
                Paragraph::new("Select a query and press Enter.").block(block),
                area,
            );
        }
    }
}

fn draw_party_overlay(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = centered_rect(60, 50, frame.area());
    let items: Vec<ListItem> = app
        .parties
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let rights = match (p.can_act_as, p.can_read_as) {
                (true, true) => "[act + read]",
                (true, false) => "[act]",
                (false, true) => "[read]",
                _ => "[none]",
            };
            let style = if i == app.selected_party {
                Style::default().fg(theme.color(Role::Accent)).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{}   {rights}", p.party)).style(style)
        })
        .collect();
    frame.render_widget(Clear, area);
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .title("Switch party · Enter select · r refresh · Esc")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.color(Role::Accent))),
        ),
        area,
    );
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
}
