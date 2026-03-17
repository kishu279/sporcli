use crate::{
    app_state::{AppState, Focus},
    events::message::AuthState,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

fn spinner_text(tick: usize, label: &str) -> String {
    let ch = SPINNER[(tick / 6) % SPINNER.len()];
    format!(" {} {}...", ch, label)
}

pub fn render(f: &mut Frame, app: &mut AppState) {
    match &app.auth_state {
        AuthState::Authenticated => {
            // User is logged in, show the player

            // match &app.status {}
            // render_main_screen(f, app, vec![], vec![], "AuthPage".to_string());

            // further page
            render_page_spotify(f, app);
        }
        AuthState::Authenticating { .. } => {
            let mut lines = vec![];

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "⏳ Waiting for authentication...",
                Style::default().fg(Color::Yellow).bold(),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "╔══════════════════════════════╗",
                Style::default().fg(Color::Green),
            )));
            lines.push(Line::from(Span::styled(
                "║  Press [c] to copy auth URL  ║",
                Style::default().fg(Color::Green).bold(),
            )));
            lines.push(Line::from(Span::styled(
                "║  Then paste it in browser    ║",
                Style::default().fg(Color::Green),
            )));
            lines.push(Line::from(Span::styled(
                "╚══════════════════════════════╝",
                Style::default().fg(Color::Green),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "URL ready to copy ✔",
                Style::default().fg(Color::DarkGray),
            )));

            let commands = vec![Line::from(vec![
                Span::styled(" [c] ", Style::default().fg(Color::Green).bold()),
                Span::styled("Copy URL  ", Style::default().fg(Color::White)),
                Span::styled(" [q] ", Style::default().fg(Color::Green).bold()),
                Span::styled("Quit", Style::default().fg(Color::White)),
            ])];

            render_main_screen(f, app, lines, commands, "Authenticating".to_string());
        }
        AuthState::NotAuthenticated => {
            // let msg = Paragraph::new("Initializing Spotify Client...").alignment(Alignment::Center);
            // f.render_widget(msg, area);
            let mut lines = vec![];

            lines.push(Line::from(""));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Auth Status: {:?}", AuthState::NotAuthenticated),
                Style::default().fg(Color::Red).bold(),
            )));

            let commands = vec![Line::from(vec![
                Span::styled(" [a] ", Style::default().fg(Color::Green).bold()),
                Span::styled("Authenticate  ", Style::default().fg(Color::White)),
                Span::styled(" [q] ", Style::default().fg(Color::Green).bold()),
                Span::styled("Quit", Style::default().fg(Color::White)),
            ])];

            render_main_screen(f, app, lines, commands, "AuthPage".to_string());
        }
        AuthState::Error(mssg) => {
            // Render an error widget instead of panicking
            let area = f.area();
            let err_msg = Paragraph::new(mssg.as_str())
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: false })
                .block(Block::bordered().title(" Error "));

            f.render_widget(err_msg, area);
        }
    }
}

fn render_main_screen(
    f: &mut Frame,
    _app: &AppState,
    line: Vec<Line<'_>>,
    commands: Vec<Line<'_>>,
    title: String,
) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let mut lines = vec![];
    lines.push(Line::from(r"  ███████╗██████╗  ██████╗ ██████╗  ██████╗"));
    lines.push(Line::from(r"  ██╔════╝██╔══██╗██╔═══██╗██╔══██╗██╔════╝"));
    lines.push(Line::from(r"  ███████╗██████╔╝██║   ██║██████╔╝██║     "));
    lines.push(Line::from(r"  ╚════██║██╔═══╝ ██║   ██║██╔══██╗██║     "));
    lines.push(Line::from(r"  ███████║██║     ╚██████╔╝██║  ██║╚██████╗"));
    lines.push(Line::from(r"  ╚══════╝╚═╝      ╚═════╝ ╚═╝  ╚═╝ ╚═════╝"));

    lines.extend(line);

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(Style::new().bold().white().on_black()),
        )
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Center);

    f.render_widget(paragraph, chunks[0]);

    if !commands.is_empty() {
        let cmd_bar = Paragraph::new(commands)
            .style(Style::default().fg(Color::White).on_black())
            .alignment(Alignment::Left);
        f.render_widget(cmd_bar, chunks[1]);
    }
}

fn render_page_spotify(f: &mut Frame, app: &mut AppState) {
    let area = f.area();

    // If there's an error message, show it in a banner at the top
    let (main_area, cmd_bar_area) = if app.error_message.is_some() {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        let err_msg = app.error_message.as_deref().unwrap_or("");
        let err_banner = Paragraph::new(Line::from(vec![
            Span::styled(" ⚠ ", Style::default().fg(Color::Red).bold()),
            Span::styled(err_msg, Style::default().fg(Color::Red)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Error ")
                .style(Style::default().fg(Color::Red)),
        );
        f.render_widget(err_banner, split[0]);

        (split[1], split[2])
    } else {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        (split[0], split[1])
    };

    let block = Block::default()
        .title("Spotify")
        .borders(Borders::ALL)
        .style(Style::new().bold().white().on_black());
    let inner = block.inner(main_area);
    f.render_widget(block, main_area);

    // Inside the block: logo on left, content on right
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(18), Constraint::Min(1)])
        .split(inner);

    // --- Left inside: subtle small logo ---
    let logo = Paragraph::new(vec![
        Line::from(Span::styled(
            " ┌─┐┌─┐┌─┐┬─┐┌─┐",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " └─┐├─┘│ │├┬┘│  ",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " └─┘┴  └─┘┴└─└─┘",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  ♫ ─────────",
            Style::default().fg(Color::Cyan),
        )),
    ])
    .alignment(Alignment::Left);
    f.render_widget(logo, cols[0]);

    // --- Right inside: panels layout ---
    // Vertical: main panels (top) | bottom bar
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(cols[1]);

    // Top: left panels | right side panel
    let panel_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(45), Constraint::Percentage(25)])
        .split(right_rows[0]);

    // Right side: track info (top half) | devices (bottom half)
    let right_side = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(panel_cols[2]);

    render_playlist_panel(f, app, panel_cols[0]);
    render_music_list_panel(f, app, panel_cols[1]);
    render_track_info_panel(f, app, right_side[0]);
    render_device_bar(f, app, right_side[1]);

    // Bottom: search box (left) | player bar (right)
    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(right_rows[1]);

    render_search_box(f, app, bottom_cols[0]);
    render_player_bar(f, app, bottom_cols[1]);

    // --- Bottom: command bar ---
    let cmd_bar = Paragraph::new(Line::from(vec![
        Span::styled(" [←/→] ", Style::default().fg(Color::Green).bold()),
        Span::styled("Prev/Next  ", Style::default().fg(Color::White)),
        Span::styled(" [space] ", Style::default().fg(Color::Green).bold()),
        Span::styled("Play/Pause  ", Style::default().fg(Color::White)),
        Span::styled(" [q] ", Style::default().fg(Color::Green).bold()),
        Span::styled("Quit", Style::default().fg(Color::White)),
    ]))
    .style(Style::default().fg(Color::White).on_black())
    .alignment(Alignment::Left);
    f.render_widget(cmd_bar, cmd_bar_area);
}

// ── Panel: Playlist ──────────────────────────────────────────────────────────
fn render_playlist_panel(f: &mut Frame, app: &mut AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::new().white().on_black())
        .title(Span::styled(" Playlist ", Style::default().fg(Color::Cyan)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let scroll = app.playlist_scroll_offset;
    let visible_height = inner.height as usize;
    app.visible_rows_playlist = visible_height.clone();

    let items: Vec<Line> = match (&app.auth_state, &app.playlist) {
        (_, Some(list)) if !list.is_empty() => list
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_height)
            .map(|(i, playlist)| {
                if i == app.selected_playlist_index {
                    Line::from(Span::styled(
                        format!(" ▶ {}", playlist.name),
                        Style::default().fg(Color::Cyan).bold(),
                    ))
                } else {
                    Line::from(Span::styled(
                        format!("   {}", playlist.name),
                        Style::default().fg(Color::White),
                    ))
                }
            })
            .collect(),
        (AuthState::Authenticated, None) => {
            vec![Line::from(Span::styled(
                spinner_text(app.tick, "Loading playlists"),
                Style::default().fg(Color::Yellow),
            ))]
        }
        _ => {
            vec![Line::from(Span::styled(
                " No playlists",
                Style::default().fg(Color::DarkGray),
            ))]
        }
    };

    f.render_widget(Paragraph::new(items), inner);
}

// ── Panel: Music List ─────────────────────────────────────────────────────────
fn render_music_list_panel(f: &mut Frame, app: &mut AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::new().white().on_black())
        .title(Span::styled(
            " Music List ",
            Style::default().fg(Color::Cyan),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let scroll = app.musiclist_scroll_offset;
    let visible = inner.height as usize;
    app.visible_rows_musiclist = visible;

    let items: Vec<Line> = match &app.auth_state {
        AuthState::Authenticated => {
            let ml = app.active_playlist_id.as_ref().and_then(|id| app.music_list.get(id));
            match ml {
                Some(ml) if !ml.items.is_empty() => ml
                    .items
                    .values()
                    .enumerate()
                    .skip(scroll)
                    .take(visible)
                    .map(|(i, track)| {
                        if i == app.selected_music_index {
                            Line::from(Span::styled(
                                format!(" ▶ {}", track.name),
                                Style::default().fg(Color::Cyan).bold(),
                            ))
                        } else {
                            Line::from(Span::styled(
                                format!("   {}", track.name),
                                Style::default().fg(Color::White),
                            ))
                        }
                    })
                    .collect(),
                None if app.active_playlist_id.is_some() => vec![Line::from(Span::styled(
                    spinner_text(app.tick, "Loading tracks"),
                    Style::default().fg(Color::Yellow),
                ))],
                _ => vec![Line::from(Span::styled(
                    " No tracks",
                    Style::default().fg(Color::DarkGray),
                ))],
            }
        }
        _ => vec![Line::from(Span::styled(
            " No tracks",
            Style::default().fg(Color::DarkGray),
        ))],
    };

    f.render_widget(Paragraph::new(items), inner);
}

// ── Panel: Track Info ────────────────────────────────────────────────────────
fn render_track_info_panel(f: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::new().white().on_black())
        .title(Span::styled(
            " Track Info ",
            Style::default().fg(Color::Cyan),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = match (&app.auth_state, &app.current_track_info) {
        (_, Some(track)) => {
            let progress = if track.duration_ms > 0 {
                (track.progress_ms * 20 / track.duration_ms) as usize
            } else {
                0
            };
            let bar: String = (0..20)
                .map(|i| if i < progress { '█' } else { '░' })
                .collect();

            vec![
                Line::from(Span::styled(
                    format!(" ♪  {}", track.name),
                    Style::default().fg(Color::Cyan).bold(),
                )),
                Line::from(Span::styled(
                    format!(" Artist: {}", track.artist),
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    format!(" Album: {}", track.album),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}", bar),
                    Style::default().fg(Color::Cyan),
                )),
            ]
        }
        (AuthState::Authenticated, None) => {
            vec![Line::from(Span::styled(
                spinner_text(app.tick, "Loading"),
                Style::default().fg(Color::Yellow),
            ))]
        }
        _ => vec![Line::from(Span::styled(
            " Nothing playing",
            Style::default().fg(Color::DarkGray),
        ))],
    };

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ── Bottom: Search Box ────────────────────────────────────────────────────────
fn render_search_box(f: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::new().white().on_black());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let (text, style) = match app.search.as_deref() {
        Some(s) => (s, Style::default().fg(Color::White)),
        None => ("Search...", Style::default().fg(Color::DarkGray)),
    };

    f.render_widget(
        Paragraph::new(Span::styled(text, style)).alignment(Alignment::Left),
        inner,
    );
}

// ── Bottom: Player Bar ────────────────────────────────────────────────────────
fn render_player_bar(f: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::new().white().on_black());

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Three controls: previous | play/pause | next (no button borders)
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(Span::styled(
            " ⏪ ",
            Style::default().fg(Color::Cyan).bold(),
        ))
        .alignment(Alignment::Center),
        sections[0],
    );

    let play_label = if !app.is_playing { " ▶ " } else { " ⏸ " };
    f.render_widget(
        Paragraph::new(Span::styled(
            play_label,
            Style::default().fg(Color::Cyan).bold(),
        ))
        .alignment(Alignment::Center),
        sections[1],
    );

    f.render_widget(
        Paragraph::new(Span::styled(
            " ⏩ ",
            Style::default().fg(Color::Cyan).bold(),
        ))
        .alignment(Alignment::Center),
        sections[2],
    );
}

// ── Bottom: Device Status Bar ────────────────────────────────────────────────
fn render_device_bar(f: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::new().white().on_black())
        .title(Span::styled(" Devices ", Style::default().fg(Color::Cyan)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = match (&app.auth_state, &app.available_devices) {
        (_, Some(devices)) if !devices.is_empty() => devices
            .iter()
            .enumerate()
            .map(|(i, device)| {
                let marker = if device.is_active { "●" } else { "○" };
                let text = format!(" {} {}", marker, device.name);
                let style = if i == app.selected_device_index {
                    match app.focus {
                        Focus::Devices => Style::default().fg(Color::Cyan).bold(),
                        _ => Style::default().fg(Color::White).bold(),
                    }
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(text, style))
            })
            .collect(),
        (AuthState::Authenticated, None) => vec![Line::from(Span::styled(
            spinner_text(app.tick, "Loading devices"),
            Style::default().fg(Color::Yellow),
        ))],
        _ => vec![Line::from(Span::styled(
            " No devices",
            Style::default().fg(Color::DarkGray),
        ))],
    };

    f.render_widget(Paragraph::new(lines).alignment(Alignment::Left), inner);
}
