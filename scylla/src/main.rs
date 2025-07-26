use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};
use ratatui::text::{Span, Line};
use ratatui::style::{Style, Color};
use crossterm::{
    execute,
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, Event, KeyCode},
};
use std::io;
use std::time::{Duration, Instant};
use rusqlite::{Connection, Result};

#[derive(Debug)]
struct Agent {
    id: String,
    hostname: String,
    ip: String,
    os: Option<String>,
    status: String,
    last_seen: Option<String>,
    location: Option<String>,
    note: Option<String>,
}

fn load_agents() -> Result<Vec<Agent>> {
    let conn = Connection::open("c2.db")?;
    let mut stmt = conn.prepare(
        "SELECT id, hostname, ip, os, status, last_seen, location, note FROM agents"
    )?;
    let agent_iter = stmt.query_map([], |row| {
        Ok(Agent {
            id: row.get(0)?,
            hostname: row.get(1)?,
            ip: row.get(2)?,
            os: row.get(3).ok(),
            status: row.get(4)?,
            last_seen: row.get(5).ok(),
            location: row.get(6).ok(),
            note: row.get(7).ok(),
        })
    })?;
    Ok(agent_iter.filter_map(Result::ok).collect())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let agents = load_agents().unwrap_or_default();
    let mut selected_index = 0usize;
    let mut list_state = ListState::default();
    list_state.select(Some(selected_index));

    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();
    let mut last_pressed: Option<KeyCode> = None; // mémorise la dernière touche pressée

    loop {
        terminal.draw(|f| {
            let size = f.area();

            let vertical_chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .margin(1)
                .constraints([
                    ratatui::layout::Constraint::Length(3),
                    ratatui::layout::Constraint::Min(10),
                    ratatui::layout::Constraint::Length(7),
                ])
                .split(size);

            let top_chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Horizontal)
                .constraints([
                    ratatui::layout::Constraint::Length(20),
                    ratatui::layout::Constraint::Min(10),
                ])
                .split(vertical_chunks[0]);

            let middle_chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Horizontal)
                .constraints([
                    ratatui::layout::Constraint::Length(30),
                    ratatui::layout::Constraint::Min(10),
                ])
                .split(vertical_chunks[1]);

            let logo_block = Block::default().title("Logo RAT").borders(ratatui::widgets::Borders::ALL);
            f.render_widget(logo_block, top_chunks[0]);

            let menu_block = Block::default().title("Menu").borders(ratatui::widgets::Borders::ALL);
            f.render_widget(menu_block, top_chunks[1]);

            let agent_items: Vec<ListItem> = agents.iter()
                .enumerate()
                .map(|(i, a)| {
                    let text = format!("{} | {} | {} | {}", a.id, a.hostname, a.ip, a.status);
                    if i == selected_index {
                        ListItem::new(Span::styled(text, Style::default().bg(Color::Blue).fg(Color::White)))
                    } else {
                        ListItem::new(Span::raw(text))
                    }
                })
                .collect();

            let agents_list = List::new(agent_items)
                .block(Block::default().title("Agent list").borders(ratatui::widgets::Borders::ALL));

            f.render_stateful_widget(agents_list, middle_chunks[0], &mut list_state);

            let datasheet_text = if let Some(agent) = agents.get(selected_index) {
                vec![
                    Line::from(format!("ID: {}", agent.id)),
                    Line::from(format!("Hostname: {}", agent.hostname)),
                    Line::from(format!("IP: {}", agent.ip)),
                    Line::from(format!("OS: {}", agent.os.as_deref().unwrap_or("-"))),
                    Line::from(format!("Status: {}", agent.status)),
                    Line::from(format!("Last seen: {}", agent.last_seen.as_deref().unwrap_or("-"))),
                    Line::from(format!("Location: {}", agent.location.as_deref().unwrap_or("-"))),
                    Line::from(format!("Note: {}", agent.note.as_deref().unwrap_or("-"))),
                ]
            } else {
                vec![Line::from("Aucun agent sélectionné")]
            };

            let datasheet = Paragraph::new(datasheet_text)
                .block(Block::default().title("Datasheet / Map").borders(ratatui::widgets::Borders::ALL));
            f.render_widget(datasheet, middle_chunks[1]);

            let terminal_block = Block::default().title("Terminal connecté").borders(ratatui::widgets::Borders::ALL);
            f.render_widget(terminal_block, vertical_chunks[2]);
        })?;

        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or(Duration::from_secs(0));
        if event::poll(timeout)? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down | KeyCode::Up => {
                        // Si c'est la même touche que la dernière, on ignore
                        if last_pressed != Some(key_event.code) {
                            match key_event.code {
                                KeyCode::Down => {
                                    if selected_index + 1 < agents.len() {
                                        selected_index += 1;
                                    }
                                }
                                KeyCode::Up => {
                                    if selected_index > 0 {
                                        selected_index -= 1;
                                    }
                                }
                                _ => {}
                            }
                            list_state.select(Some(selected_index));
                            last_pressed = Some(key_event.code); // on note la touche
                        }
                    }
                    _ => {
                        last_pressed = None; // autre touche => reset
                    }
                }
            }
        } else {
            last_pressed = None; // pas d'événement => reset
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
