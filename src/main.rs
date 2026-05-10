use std::io;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use sysinfo::System;

fn main() -> std::io::Result<()> {
    let mut system = System::new_all();

    system.refresh_all();

    enable_raw_mode()?;

    let mut stdout: io::Stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend: CrosstermBackend<io::Stdout> = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(area);

            let title: Paragraph<'_> = Paragraph::new("Mi app con ratatuii")
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Contenido"));

            let mut lineas = Vec::new();

            for (pid, process) in system.processes() {
                lineas.push(Line::from(format!(
                    "PID: {:?} | Nombre: {:?} | CPU: {:.2}% | RAM: {} KB",
                    pid,
                    process.name(),
                    process.cpu_usage(),
                    process.memory()
                )))
            }
            let content = Paragraph::new(lineas)
                .block(Block::default().borders(Borders::ALL).title("CONTENIDO"));

            frame.render_widget(title, chunks[0]);
            frame.render_widget(content, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                break;
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
