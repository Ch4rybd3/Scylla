// Import des modules essentiels de ratatui
use ratatui::{
    backend::CrosstermBackend, // Backend pour dessiner via crossterm
    Terminal,                  // Terminal = moteur principal pour dessiner
};
use ratatui::widgets::{Block, List, ListItem, Paragraph};
use ratatui::text::{Span, Line}; // Pour gérer le texte et les lignes
// Import des modules crossterm pour gérer le terminal et les touches
use crossterm::{
    execute, // Pour exécuter des commandes dans le terminal
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, Event, KeyCode}, // Pour lire les événements clavier
};
use std::io; // Pour l’entrée/sortie
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

    // Collecter dans un Vec<Agent>
    let agents: Vec<Agent> = agent_iter.filter_map(Result::ok).collect();

    Ok(agents)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Activer le mode brut du terminal : désactive l’écho clavier, etc.
    enable_raw_mode()?;
    // Passer en "écran alternatif" : on dessine sur un écran séparé
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    // Créer le backend crossterm
    let backend = CrosstermBackend::new(stdout);
    // Initialiser le terminal ratatui avec ce backend
    let mut terminal = Terminal::new(backend)?;

    let agents = load_agents().unwrap_or_default();

    // Boucle principale : on dessine sans cesse jusqu’à ce que l’utilisateur appuie sur 'q'
    loop {
        // Dessiner une "frame"
        terminal.draw(|f| {
            let size = f.area();

            // Découpe en 3 lignes verticales : haut, milieu, bas
            let vertical_chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .margin(1)
                .constraints([
                    ratatui::layout::Constraint::Length(3), // Ligne du haut (logo + menu)
                    ratatui::layout::Constraint::Min(10),   // Ligne du milieu
                    ratatui::layout::Constraint::Length(7), // Ligne du bas (terminal)
                ])
                .split(size);

            // Ligne du haut : logo + menu
            let top_chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Horizontal)
                .constraints([
                    ratatui::layout::Constraint::Length(20),  // Logo à gauche
                    ratatui::layout::Constraint::Min(10),     // Menu tabs à droite
                ])
                .split(vertical_chunks[0]);

            // Ligne du milieu : liste + datasheet/map
            let middle_chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Horizontal)
                .constraints([
                    ratatui::layout::Constraint::Length(30),  // Liste des agents à gauche
                    ratatui::layout::Constraint::Min(10),     // Datasheet et map à droite
                ])
                .split(vertical_chunks[1]);

            // Dessiner les blocs (pour l’instant, juste Block::default() + titre)
            let logo_block = ratatui::widgets::Block::default()
                .title("Logo RAT")
                .borders(ratatui::widgets::Borders::ALL);
            f.render_widget(logo_block, top_chunks[0]);

            let menu_block = ratatui::widgets::Block::default()
                .title("Menu")
                .borders(ratatui::widgets::Borders::ALL);
            f.render_widget(menu_block, top_chunks[1]);

            let agent_items: Vec<ListItem> = agents.iter()
                .map(|a| {
                    let text = format!("{} | {} | {} | {}", a.id, a.hostname, a.ip, a.status);
                    ListItem::new(text)
                })
                .collect();

            // Créer la liste avec un Block
            let agents_list = List::new(agent_items)
                .block(ratatui::widgets::Block::default()
                    .title("Agent list")
                    .borders(ratatui::widgets::Borders::ALL));

            f.render_widget(agents_list, middle_chunks[0]);

            let datasheet_text = if let Some(agent) = agents.get(0) {
                vec![
                    Line::from(vec![Span::raw(format!("ID: {}", agent.id))]),
                    Line::from(vec![Span::raw(format!("Hostname: {}", agent.hostname))]),
                    Line::from(vec![Span::raw(format!("IP: {}", agent.ip))]),
                    Line::from(vec![Span::raw(format!("OS: {}", agent.os.as_deref().unwrap_or("-")))]),
                    Line::from(vec![Span::raw(format!("Status: {}", agent.status))]),
                    Line::from(vec![Span::raw(format!("Last seen: {}", agent.last_seen.as_deref().unwrap_or("-")))]),
                    Line::from(vec![Span::raw(format!("Location: {}", agent.location.as_deref().unwrap_or("-")))]),
                    Line::from(vec![Span::raw(format!("Note: {}", agent.note.as_deref().unwrap_or("-")))]),
                ]
            } else {
                vec![Line::from(vec![Span::raw("Aucun agent sélectionné")])]
            };

            let datasheet = Paragraph::new(datasheet_text)
                .block(Block::default()
                    .title("Datasheet / Map")
                    .borders(ratatui::widgets::Borders::ALL));
            f.render_widget(datasheet, middle_chunks[1]);

            let terminal_block = ratatui::widgets::Block::default()
                .title("Terminal connecté")
                .borders(ratatui::widgets::Borders::ALL);
            f.render_widget(terminal_block, vertical_chunks[2]);
        })?;


        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') { // on key "q" press, exit the program
                break;
            }
        }
    }

    // Avant de quitter : désactiver le mode brut
    disable_raw_mode()?;
    // Quitter l’écran alternatif pour revenir au terminal normal
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    // Afficher à nouveau le curseur
    terminal.show_cursor()?;

    Ok(())
}
