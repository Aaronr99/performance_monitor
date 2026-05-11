use std::{
    io,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
    vec,
};

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
    widgets::{Block, Borders, Paragraph, Sparkline},
};

use sysinfo::System;

struct ProcessInfo {
    pid: String,
    name: String,
    cpu: f32,
    memory: u64,
}

impl ProcessInfo {
    fn new(pid: String, name: String, cpu: f32, memory: u64) -> Self {
        Self {
            pid,
            name,
            cpu,
            memory,
        }
    }

    fn memory_mb(&self) -> f64 {
        self.memory as f64 / 1024.0 / 1024.0
    }
}

const CANTIDAD_HISTORIAL: usize = 200;

fn main() -> std::io::Result<()> {
    //let mut system = System::new_all();

    enable_raw_mode()?;

    let mut stdout: io::Stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend: CrosstermBackend<io::Stdout> = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let frame_duration = Duration::from_millis(16);

    let mut vector_procesos_actual: Vec<ProcessInfo> = Vec::new();

    let mut vector_procesos_tiempo: Vec<(String, Vec<u64>)> = Vec::new();

    let mut lista_pid_modificados: Vec<String> = Vec::new();

    let (tx, rx) = mpsc::channel::<Vec<ProcessInfo>>();

    thread::spawn(move || {
        let mut system = System::new_all();

        loop {
            system.refresh_all();

            let mut procesos = Vec::new();

            for (pid, process) in system.processes() {
                procesos.push(ProcessInfo::new(
                    pid.to_string(),
                    process.name().to_str().unwrap_or("").to_string(),
                    process.cpu_usage(),
                    process.memory(),
                ));
            }

            if tx.send(procesos).is_err() {
                break;
            }

            thread::sleep(Duration::from_millis(500));
        }
    });

    loop {
        let start = Instant::now();

        if let Ok(procesos) = rx.try_recv() {
            vector_procesos_actual = procesos;
            lista_pid_modificados.clear();

            // Si esta vacia la inicializo con los valores actuales
            if vector_procesos_tiempo.is_empty() {
                for proceso in &vector_procesos_actual {
                    let mut vector_tiempos: Vec<u64> = vec![0; CANTIDAD_HISTORIAL - 1];
                    vector_tiempos.insert(0, proceso.cpu.clone() as u64);

                    vector_procesos_tiempo.push((proceso.pid.clone(), vector_tiempos));
                }
            } else {
                for proceso in &vector_procesos_actual {
                    let elemento_en_tiempo = vector_procesos_tiempo
                        .iter_mut()
                        .find(|x| x.0 == proceso.pid);

                    if elemento_en_tiempo.is_none() {
                        let mut vector_tiempos: Vec<u64> = vec![0; CANTIDAD_HISTORIAL - 1];
                        vector_tiempos.insert(0, proceso.cpu.clone() as u64);

                        vector_procesos_tiempo.push((proceso.pid.clone(), vector_tiempos));
                    } else {
                        if let Some(elemento) = elemento_en_tiempo {
                            elemento.1.insert(0, proceso.cpu as u64);
                            if elemento.1.len() > CANTIDAD_HISTORIAL {
                                elemento.1.pop();
                            }
                        }
                    }

                    lista_pid_modificados.push(proceso.pid.clone());
                }

                vector_procesos_tiempo.retain(|x| lista_pid_modificados.contains(&x.0));

                vector_procesos_tiempo.sort_by(|a, b| b.1[0].partial_cmp(&a.1[0]).unwrap());
            }
        }

        terminal.draw(|frame| {
            let area = frame.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                ])
                .split(area);

            let texto_titulo = format!(
                "Cantidad Procesos Activos: {}",
                vector_procesos_actual.len()
            );

            let title: Paragraph<'_> = Paragraph::new(texto_titulo)
                .style(Style::default().fg(Color::Cyan))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Performance Monitor"),
                );

            frame.render_widget(title, chunks[0]);

            for (i, proceso) in vector_procesos_tiempo.iter().take(10).enumerate() {
                let detalles_proceso = vector_procesos_actual
                    .iter()
                    .find(|x| x.pid.as_str() == proceso.0.as_str());

                let max_cpu_usage = proceso
                    .1
                    .iter()
                    .copied()
                    .max()
                    .expect("ERROR AL OBTENER EL MAXIMO");

                if let Some(detalle) = detalles_proceso {
                    let color = if detalle.name == "performance_monitor.exe" {
                        Color::Green
                    } else {
                        Color::Gray
                    };

                    let sparkline = Sparkline::default()
                        .block(Block::default().borders(Borders::ALL).title(format!(
                            "PID: {} Nombre Proceso: {} Ram Usada: {:.2} Cpu Usada: {:.2}",
                            &proceso.0,
                            detalle.name,
                            detalle.memory_mb(),
                            detalle.cpu
                        )).border_style(Style::default().fg(color) ))
                        .data(&proceso.1)
                        .max(max_cpu_usage)
                        .style(Style::default().fg(color));

                    frame.render_widget(sparkline, chunks[i + 1]);
                }
            }
        })?;

        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        let elapsed = start.elapsed();

        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
