use chrono::Local;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line,Span},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: u32,
    description: String,
    created_at: String,
    due_date: String,
    status: String, // "TODO", "DOING", "DONE"
}

#[derive(Debug, Default)]
struct KanbanBoard {
    tasks: Vec<Task>,
    selected_status: usize, // Index of the currently selected status
    selected_task: usize,   // Index of the currently selected task
}

impl KanbanBoard {
    fn new() -> Self {
        Self {
            tasks: Vec::new(),
            selected_status: 0,
            selected_task: 0,
        }
    }

    fn save_to_file(&self) {
        let file_path = "kanban_board.json";
        if let Err(err) = fs::write(
            file_path,
            serde_json::to_string_pretty(&self.tasks).unwrap(),
        ) {
            eprintln!("Failed to save tasks: {}", err);
        }
    }

    fn load_from_file(&mut self) {
        let file_path = "kanban_board.json";
        if Path::new(file_path).exists() {
            if let Ok(data) = fs::read_to_string(file_path) {
                self.tasks = serde_json::from_str(&data).unwrap_or_else(|_| Vec::new());
            }
        }
    }

    fn add_task(&mut self, description: String, due_date: String) {
        let id = self.tasks.len() as u32 + 1;
        let created_at = Local::now().format("%Y-%m-%d").to_string();
        let task = Task {
            id,
            description,
            created_at,
            due_date,
            status: "TODO".to_string(),
        };
        self.tasks.push(task);
    }

    fn move_task(&mut self, direction: isize) {
        let statuses = ["TODO", "DOING", "DONE"];
        if let Some(task) = self
            .tasks
            .iter_mut()
            .find(|t| t.status == statuses[self.selected_status])
        {
            let new_status_index = (self.selected_status as isize + direction)
                .clamp(0, statuses.len() as isize - 1) as usize;
            task.status = statuses[new_status_index].to_string();
        }
    }

    fn get_tasks_by_status(&self, status: &str) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.status == status).collect()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Kanban board setup
    let mut board = KanbanBoard::new();
    board.load_from_file();

    let result = run_app(&mut terminal, &mut board);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Save tasks
    board.save_to_file();

    if let Err(err) = result {
        eprintln!("{:?}", err);
    }
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, board: &mut KanbanBoard) -> io::Result<()> {
    let statuses = ["TODO", "DOING", "DONE"];
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(34),
                ])
                .split(f.area());

            for (i, status) in statuses.iter().enumerate() {
                let tasks = board.get_tasks_by_status(status);
                let items: Vec<ListItem> = tasks
                    .iter()
                    .map(|t| {
                        ListItem::new(Line::from(vec![
                            Span::raw(format!("[#{}] ", t.id)),
                            Span::styled(&t.description, Style::default().fg(Color::White)),
                            Span::raw(format!(" (Due: {})", t.due_date)),
                        ]))
                    })
                    .collect();

                let list = List::new(items)
                    .block(
                        Block::default()
                            .title(*status)
                            .borders(Borders::ALL)
                            .border_style(if board.selected_status == i {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default()
                            }),
                    )
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD));

                f.render_widget(list, chunks[i]);
            }
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()), // Quit
                KeyCode::Char('a') => {
                    // Add a new task
                    if let Some(description) = prompt("Enter task description") {
                        if let Some(due_date) = prompt("Enter due date (YYYY-MM-DD)") {
                            board.add_task(description, due_date);
                        }
                    }
                }
                KeyCode::Left => {
                    if board.selected_status > 0 {
                        board.selected_status -= 1;
                    }
                }
                KeyCode::Right => {
                    if board.selected_status < statuses.len() - 1 {
                        board.selected_status += 1;
                    }
                }
                KeyCode::Up => {
                    if board.selected_task > 0 {
                        board.selected_task -= 1;
                    }
                }
                KeyCode::Down => {
                    let max_tasks = board
                        .get_tasks_by_status(statuses[board.selected_status])
                        .len();
                    if board.selected_task < max_tasks - 1 {
                        board.selected_task += 1;
                    }
                }
                KeyCode::Enter => {
                    board.move_task(1); // Move to the next status
                }
                _ => {}
            }
        }
    }
}

fn prompt(message: &str) -> Option<String> {
    disable_raw_mode().ok();
    println!("{}", message);
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        enable_raw_mode().ok();
        Some(input.trim().to_string())
    } else {
        enable_raw_mode().ok();
        None
    }
}
