use crate::kanban_board::KanbanBoard;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};
use std::io;

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, board: &mut KanbanBoard) -> io::Result<()> {
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
