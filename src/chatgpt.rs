use clap::{Arg, Command};

fn main() -> io::Result<()> {
    let matches = Command::new("kanban")
        .version("1.0")
        .author("Your Name")
        .about("Command-line Kanban board application")
        .subcommand(Command::new("init").about("Initialize Kanban in the current directory"))
        .subcommand(Command::new("add")
            .about("Add a new task")
            .arg(Arg::new("status").required(true).help("Task status"))
            .arg(Arg::new("tag").required(true).help("Task tag"))
            .arg(Arg::new("description").required(true).help("Task description")))
        .subcommand(Command::new("show").about("Show the Kanban board"))
        .subcommand(Command::new("list").about("List all tasks"))
        .subcommand(Command::new("tags").about("List all tags"))
        .subcommand(Command::new("stats").about("Show statistics"))
        .get_matches();

    // Handle subcommands
    match matches.subcommand() {
        Some(("init", _)) => {
            KanbanApp::init(".")?;
        }
        Some(("add", sub_matches)) => {
            let status = sub_matches.get_one::<String>("status").unwrap();
            let tag = sub_matches.get_one::<String>("tag").unwrap();
            let description = sub_matches.get_one::<String>("description").unwrap();
            KanbanApp::add_task_interactive(status, tag, description)?;
        }
        Some(("show", _)) => {
            let mut app = KanbanApp::new("kanban_config.json");
            app.show_board();
        }
        Some(("list", _)) => {
            let mut app = KanbanApp::new("kanban_config.json");
            app.list_tasks();
        }
        Some(("tags", _)) => {
            let mut app = KanbanApp::new("kanban_config.json");
            app.list_tags();
        }
        Some(("stats", _)) => {
            let mut app = KanbanApp::new("kanban_config.json");
            app.show_stats();
        }
        _ => {
            println!("Use --help for available commands.");
        }
    }
    Ok(())
}


use csv::Writer;

impl KanbanApp {
    const CSV_FILE: &'static str = ".kanban.csv";

    fn save_to_csv(&self) -> io::Result<()> {
        let mut writer = Writer::from_path(Self::CSV_FILE)?;
        writer.write_record(&["id", "status", "tag", "description", "created_at", "due_date"])?;
        for column in &self.columns {
            for task in &column.tasks {
                writer.write_record(&[
                    task.id.to_string(),
                    column.name.clone(),
                    task.tags.join(", "),
                    task.description.clone().unwrap_or_default(),
                    task.created_at.to_string(),
                    task.due_date.map_or("".to_string(), |d| d.to_string()),
                ])?;
            }
        }
        writer.flush()?;
        Ok(())
    }

    fn load_from_csv() -> io::Result<Vec<Column>> {
        let mut columns: Vec<Column> = vec![];
        if !Path::new(Self::CSV_FILE).exists() {
            return Ok(columns);
        }

        let mut reader = csv::Reader::from_path(Self::CSV_FILE)?;
        for result in reader.records() {
            let record = result?;
            let column_name = record.get(1).unwrap().to_string();
            let task = Task {
                id: Uuid::parse_str(record.get(0).unwrap())?,
                title: record.get(3).unwrap().to_string(),
                description: Some(record.get(3).unwrap().to_string()),
                tags: record.get(2).unwrap().split(", ").map(String::from).collect(),
                created_at: record.get(4).unwrap().parse().unwrap(),
                due_date: record.get(5).and_then(|d| d.parse().ok()),
            };

            let column = columns.iter_mut().find(|c| c.name == column_name);
            if let Some(col) = column {
                col.tasks.push(task);
            } else {
                columns.push(Column {
                    name: column_name,
                    tasks: vec![task],
                });
            }
        }
        Ok(columns)
    }
}

impl KanbanApp {
    fn list_tasks(&self) {
        for column in &self.columns {
            println!("{}:", column.name);
            for task in &column.tasks {
                println!(
                    "- [{}] {} ({})",
                    task.id,
                    task.title,
                    task.tags.join(", ")
                );
            }
        }
    }

    fn list_tags(&self) {
        let tags: Vec<String> = self
            .columns
            .iter()
            .flat_map(|c| c.tasks.iter().flat_map(|t| t.tags.clone()))
            .collect();
        let unique_tags: Vec<_> = tags.into_iter().collect();
        println!("Tags: {:?}", unique_tags);
    }

    fn show_stats(&self) {
        let mut status_counts = std::collections::HashMap::new();
        for column in &self.columns {
            status_counts
                .entry(&column.name)
                .and_modify(|c| *c += column.tasks.len())
                .or_insert(column.tasks.len());
        }
        for (status, count) in status_counts {
            println!("{}: {}", status, count);
        }
    }
}

impl KanbanApp {
    fn init(dir: &str) -> io::Result<()> {
        let config_path = format!("{}/.kanban_config.json", dir);
        if Path::new(&config_path).exists() {
            println!("Kanban already initialized in this directory.");
            return Ok(());
        }
        let config = Config::default();
        fs::create_dir_all(dir)?;
        fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
        fs::File::create(format!("{}/.kanban.csv", dir))?;
        println!("Kanban initialized in {}", dir);
        Ok(())
    }
}
