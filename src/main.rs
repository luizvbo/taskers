use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::{DateTime, Local, Utc};
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::Path;
use std::{fs, io};
use uuid::Uuid;

fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match date {
        Some(date) => serializer.serialize_i64(date.timestamp()),
        None => serializer.serialize_none(),
    }
}

fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    // Deserialize as an `Option<i64>` (timestamp in seconds)
    let timestamp: Option<i64> = Option::deserialize(deserializer)?;

    // Convert timestamp to `DateTime<Utc>` if available
    Ok(timestamp.and_then(|ts| {
        NaiveDateTime::from_timestamp_opt(ts, 0) 
            .map(|naive| DateTime::<Utc>::from_utc(naive, Utc)) 
    }))
}

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    id: Uuid,
    title: String,
    description: Option<String>,
    tags: Vec<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
    due_date: Option<DateTime<Utc>>,
}

impl Task {
    fn new(
        title: &str,
        description: Option<String>,
        tags: Vec<String>,
        due_date: Option<DateTime<Local>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.to_string(),
            description,
            tags,
            created_at: Local::now().into(),
            due_date: due_date.map(|dt| dt.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Column {
    name: String,
    tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize)]
struct Config {
    columns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            columns: vec!["Todo".to_string(), "Doing".to_string(), "Done".to_string()],
        }
    }
}

struct KanbanApp {
    columns: Vec<Column>,
    config_path: String,
    selected_column: usize,
    selected_task: Option<usize>,
}

impl KanbanApp {
    fn new(config_path: &str) -> Self {
        let config = KanbanApp::load_config(config_path).unwrap_or_default();
        let columns = config
            .columns
            .into_iter()
            .map(|name| Column {
                name,
                tasks: Vec::new(),
            })
            .collect();

        Self {
            columns,
            config_path: config_path.to_string(),
            selected_column: 0,
            selected_task: None,
        }
    }

    fn load_config(path: &str) -> io::Result<Config> {
        if Path::new(path).exists() {
            let data = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            Ok(Config::default())
        }
    }

    fn save_config(&self) -> io::Result<()> {
        let config = Config {
            columns: self.columns.iter().map(|col| col.name.clone()).collect(),
        };
        let data = serde_json::to_string_pretty(&config)?;
        fs::write(&self.config_path, data)
    }

    fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| self.draw_ui(f))?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Left => self.previous_column(),
                    KeyCode::Right => self.next_column(),
                    KeyCode::Up => self.previous_task(),
                    KeyCode::Down => self.next_task(),
                    KeyCode::Char('a') => self.add_task(),
                    KeyCode::Char('e') => self.edit_task(),
                    KeyCode::Char('m') => self.move_task(),
                    _ => {}
                }
            }
        }

        disable_raw_mode()
    }

    fn draw_ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                self.columns
                    .iter()
                    .map(|_| Constraint::Percentage(100 / self.columns.len() as u16))
                    .collect::<Vec<_>>(),
            )
            .split(f.area()); // Use f.area() instead of f.size()

        for (i, column) in self.columns.iter().enumerate() {
            let block = Block::default().title(&*column.name).borders(Borders::ALL);

            let items: Vec<ListItem> = column
                .tasks
                .iter()
                .map(|task| ListItem::new(task.title.clone()))
                .collect();
            let list = List::new(items).block(block);

            f.render_widget(list, chunks[i]);
        }
    }

    fn previous_column(&mut self) {
        if self.selected_column > 0 {
            self.selected_column -= 1;
        }
    }

    fn next_column(&mut self) {
        if self.selected_column < self.columns.len() - 1 {
            self.selected_column += 1;
        }
    }

    fn previous_task(&mut self) {
        if let Some(selected_task) = self.selected_task {
            if selected_task > 0 {
                self.selected_task = Some(selected_task - 1);
            }
        }
    }

    fn next_task(&mut self) {
        if let Some(selected_task) = self.selected_task {
            if selected_task < self.columns[self.selected_column].tasks.len() - 1 {
                self.selected_task = Some(selected_task + 1);
            }
        }
    }

    fn add_task(&mut self) {
        let title = self.input("Enter task title: ");
        let description = self.input("Enter task description (optional): ");
        let tags = self.input("Enter tags, separated by commas: ");
        let tags: Vec<String> = tags.split(',').map(|s| s.trim().to_string()).collect();

        let due_date = self.input("Enter due date (YYYY-MM-DD, optional): ");
        let due_date = due_date.parse::<chrono::NaiveDate>().ok().map(|date| {
            Local
                .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
                .unwrap()
                .into()
        });

        let task = Task::new(&title, Some(description), tags, due_date);
        self.columns[self.selected_column].tasks.push(task);
    }

    fn edit_task(&mut self) {
        // Editing logic here
    }

    fn move_task(&mut self) {
        // Moving logic here
    }

    fn input(&self, prompt: &str) -> String {
        disable_raw_mode().unwrap();
        println!("{}", prompt);
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        enable_raw_mode().unwrap();
        input.trim().to_string()
    }
}

fn main() -> io::Result<()> {
    let config_path = "kanban_config.json";
    let mut app = KanbanApp::new(config_path);
    app.run()
}
