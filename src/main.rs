//! MDBY CLI - Markdown Database

use clap::{Parser, Subcommand, ValueEnum};
use mdby::{Database, Document, QueryResult};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mdby")]
#[command(about = "A markdown-based git-backed database", long_about = None)]
#[command(version)]
struct Cli {
    /// Database directory (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    database: PathBuf,

    /// Output format
    #[arg(short, long, default_value = "table", global = true)]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Human-readable table format
    Table,
    /// JSON format
    Json,
    /// Minimal format (just values)
    Minimal,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new MDBY database
    Init,

    /// Execute an MDQL query
    Query {
        /// The MDQL query to execute
        query: String,
    },

    /// Start interactive REPL mode
    Repl,

    /// Regenerate all views
    Regenerate,

    /// Sync with remote git repository
    Sync {
        /// Remote name (default: origin)
        #[arg(default_value = "origin")]
        remote: String,
    },

    /// Show database status
    Status,

    /// List collections
    Collections,

    /// List views
    Views,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging (only if RUST_LOG is set)
    if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::fmt::init();
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => init_database(&cli.database).await,
        Commands::Query { query } => execute_query(&cli.database, &query, cli.format).await,
        Commands::Repl => run_repl(&cli.database).await,
        Commands::Regenerate => regenerate_views(&cli.database).await,
        Commands::Sync { remote } => sync_database(&cli.database, &remote).await,
        Commands::Status => show_status(&cli.database).await,
        Commands::Collections => list_collections(&cli.database, cli.format).await,
        Commands::Views => list_views(&cli.database, cli.format).await,
    };

    if let Err(e) = result {
        // Print user-friendly error message
        eprintln!("Error: {}", e);

        // Check if we have an MDBY error with a suggestion
        if let Some(mdby_err) = e.downcast_ref::<mdby::Error>() {
            if let Some(suggestion) = mdby_err.suggestion() {
                eprintln!("Hint: {}", suggestion);
            }
        }

        std::process::exit(1);
    }

    Ok(())
}

async fn init_database(path: &PathBuf) -> anyhow::Result<()> {
    println!("Initializing MDBY database at {:?}...", path);

    // Create the database (this will init git if needed)
    let _db = Database::open(path).await?;

    // Create standard directories
    tokio::fs::create_dir_all(path.join("collections")).await?;
    tokio::fs::create_dir_all(path.join("views")).await?;
    tokio::fs::create_dir_all(path.join(".mdby/schemas")).await?;
    tokio::fs::create_dir_all(path.join(".mdby/views")).await?;
    tokio::fs::create_dir_all(path.join(".mdby/templates")).await?;

    println!("Database initialized successfully!");
    println!();
    println!("Directory structure:");
    println!("  collections/     - Your data collections");
    println!("  views/           - Generated view outputs");
    println!("  .mdby/schemas/   - Collection schemas");
    println!("  .mdby/views/     - View definitions");
    println!("  .mdby/templates/ - HTML templates for views");
    println!();
    println!("Get started:");
    println!("  mdby query \"CREATE COLLECTION todos (title STRING REQUIRED, done BOOL DEFAULT false)\"");
    println!("  mdby query \"INSERT INTO todos (id, title) VALUES ('task-1', 'Hello MDBY!')\"");
    println!("  mdby query \"SELECT * FROM todos\"");

    Ok(())
}

async fn execute_query(path: &PathBuf, query: &str, format: OutputFormat) -> anyhow::Result<()> {
    let mut db = Database::open(path).await?;
    let result = db.execute(query).await?;

    match result {
        QueryResult::Documents(docs) => {
            print_documents(&docs, format);
        }
        QueryResult::Affected(count) => {
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::json!({"affected": count}));
                }
                _ => {
                    println!("{} document(s) affected.", count);
                }
            }
        }
        QueryResult::CollectionCreated(name) => {
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::json!({"created": "collection", "name": name}));
                }
                _ => {
                    println!("Collection '{}' created.", name);
                }
            }
        }
        QueryResult::ViewCreated(name) => {
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::json!({"created": "view", "name": name}));
                }
                _ => {
                    println!("View '{}' created.", name);
                }
            }
        }
        QueryResult::Collections(names) => {
            print_list("Collections", &names, format);
        }
        QueryResult::Views(names) => {
            print_list("Views", &names, format);
        }
    }

    Ok(())
}

fn print_list(label: &str, items: &[String], format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&items).unwrap_or_default());
        }
        OutputFormat::Table => {
            if items.is_empty() {
                println!("No {} found.", label.to_lowercase());
            } else {
                println!("{}:", label);
                for name in items {
                    println!("  {}", name);
                }
                println!("\n({} total)", items.len());
            }
        }
        OutputFormat::Minimal => {
            for name in items {
                println!("{}", name);
            }
        }
    }
}

fn print_documents(docs: &[Document], format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let json_docs: Vec<serde_json::Value> = docs.iter().map(doc_to_json).collect();
            println!("{}", serde_json::to_string_pretty(&json_docs).unwrap_or_default());
        }
        OutputFormat::Table => {
            if docs.is_empty() {
                println!("No documents found.");
                return;
            }

            // Collect all field names
            let mut all_fields: Vec<String> = vec!["id".to_string()];
            for doc in docs {
                for key in doc.fields.keys() {
                    if !all_fields.contains(key) {
                        all_fields.push(key.clone());
                    }
                }
            }

            // Calculate column widths
            let mut widths: HashMap<&str, usize> = HashMap::new();
            for field in &all_fields {
                widths.insert(field, field.len());
            }
            for doc in docs {
                let id_len = doc.id.len();
                if id_len > *widths.get("id").unwrap_or(&0) {
                    widths.insert("id", id_len);
                }
                for (key, value) in &doc.fields {
                    let val_str = format_value(value);
                    let len = val_str.len();
                    if len > *widths.get(key.as_str()).unwrap_or(&0) {
                        widths.insert(key, len);
                    }
                }
            }

            // Print header
            let header: Vec<String> = all_fields
                .iter()
                .map(|f| format!("{:width$}", f, width = widths.get(f.as_str()).unwrap_or(&0)))
                .collect();
            println!("{}", header.join(" | "));

            // Print separator
            let sep: Vec<String> = all_fields
                .iter()
                .map(|f| "-".repeat(*widths.get(f.as_str()).unwrap_or(&0)))
                .collect();
            println!("{}", sep.join("-+-"));

            // Print rows
            for doc in docs {
                let row: Vec<String> = all_fields
                    .iter()
                    .map(|f| {
                        let val = if f == "id" {
                            doc.id.clone()
                        } else {
                            doc.fields.get(f).map(format_value).unwrap_or_default()
                        };
                        format!("{:width$}", val, width = widths.get(f.as_str()).unwrap_or(&0))
                    })
                    .collect();
                println!("{}", row.join(" | "));
            }

            println!("\n({} row(s))", docs.len());
        }
        OutputFormat::Minimal => {
            for doc in docs {
                println!("{}", doc.id);
            }
        }
    }
}

fn format_value(value: &mdby::storage::document::Value) -> String {
    use mdby::storage::document::Value;
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(_) => "{...}".to_string(),
    }
}

fn doc_to_json(doc: &Document) -> serde_json::Value {
    use mdby::storage::document::Value;

    fn value_to_json(v: &Value) -> serde_json::Value {
        match v {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Int(i) => serde_json::json!(i),
            Value::Float(f) => serde_json::json!(f),
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Array(arr) => serde_json::Value::Array(arr.iter().map(value_to_json).collect()),
            Value::Object(obj) => {
                let map: serde_json::Map<String, serde_json::Value> =
                    obj.iter().map(|(k, v)| (k.clone(), value_to_json(v))).collect();
                serde_json::Value::Object(map)
            }
        }
    }

    let mut obj = serde_json::Map::new();
    obj.insert("id".to_string(), serde_json::Value::String(doc.id.clone()));

    for (key, value) in &doc.fields {
        obj.insert(key.clone(), value_to_json(value));
    }

    if !doc.body.is_empty() {
        obj.insert("_body".to_string(), serde_json::Value::String(doc.body.clone()));
    }

    serde_json::Value::Object(obj)
}

async fn run_repl(path: &PathBuf) -> anyhow::Result<()> {
    use std::io::{self, BufRead, Write};

    println!("MDBY Interactive Shell");
    println!("Type 'help' for commands, 'exit' to quit.");
    println!();

    let mut db = Database::open(path).await?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("mdql> ");
        stdout.flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line.to_lowercase().as_str() {
            "exit" | "quit" | "\\q" => break,
            "help" | "\\h" => {
                println!("Commands:");
                println!("  SELECT * FROM <collection>    - Query documents");
                println!("  INSERT INTO <collection> ...  - Insert a document");
                println!("  UPDATE <collection> SET ...   - Update documents");
                println!("  DELETE FROM <collection> ...  - Delete documents");
                println!("  CREATE COLLECTION <name> ...  - Create a collection");
                println!("  CREATE VIEW <name> AS ...     - Create a view");
                println!();
                println!("Special:");
                println!("  help, \\h  - Show this help");
                println!("  exit, \\q  - Exit the shell");
                continue;
            }
            _ => {}
        }

        match db.execute(line).await {
            Ok(result) => match result {
                QueryResult::Documents(docs) => {
                    print_documents(&docs, OutputFormat::Table);
                }
                QueryResult::Affected(n) => println!("({} row(s) affected)", n),
                QueryResult::CollectionCreated(name) => println!("Collection '{}' created", name),
                QueryResult::ViewCreated(name) => println!("View '{}' created", name),
                QueryResult::Collections(names) => {
                    print_list("Collections", &names, OutputFormat::Table);
                }
                QueryResult::Views(names) => {
                    print_list("Views", &names, OutputFormat::Table);
                }
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                if let Some(mdby_err) = e.downcast_ref::<mdby::Error>() {
                    if let Some(suggestion) = mdby_err.suggestion() {
                        eprintln!("Hint: {}", suggestion);
                    }
                }
            }
        }
        println!();
    }

    println!("Goodbye!");
    Ok(())
}

async fn regenerate_views(path: &PathBuf) -> anyhow::Result<()> {
    let db = Database::open(path).await?;
    println!("Regenerating views...");
    db.regenerate_views().await?;
    println!("Done!");
    Ok(())
}

async fn sync_database(path: &PathBuf, remote: &str) -> anyhow::Result<()> {
    let mut db = Database::open(path).await?;
    println!("Syncing with {}...", remote);
    let result = db.sync().await?;
    println!("Pulled: {} commits", result.pulled);
    println!("Pushed: {} commits", result.pushed);
    if !result.conflicts_resolved.is_empty() {
        println!("Resolved conflicts:");
        for path in &result.conflicts_resolved {
            println!("  - {}", path);
        }
    }
    Ok(())
}

async fn show_status(path: &PathBuf) -> anyhow::Result<()> {
    let db = Database::open(path).await?;

    println!("MDBY Database Status");
    println!("====================");
    println!("Path: {:?}", db.root);
    println!();

    // Count collections
    let collections_path = path.join("collections");
    if collections_path.exists() {
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(&collections_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.path().is_dir() {
                count += 1;
            }
        }
        println!("Collections: {}", count);
    } else {
        println!("Collections: 0");
    }

    // Count views
    let views_path = path.join(".mdby/views");
    if views_path.exists() {
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(&views_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.path().extension().map(|e| e == "yaml").unwrap_or(false) {
                count += 1;
            }
        }
        println!("Views: {}", count);
    } else {
        println!("Views: 0");
    }

    // Git status
    if db.git.has_changes()? {
        println!("\nUncommitted changes detected.");
    } else {
        println!("\nNo uncommitted changes.");
    }

    Ok(())
}

async fn list_collections(path: &PathBuf, format: OutputFormat) -> anyhow::Result<()> {
    let collections_path = path.join("collections");

    if !collections_path.exists() {
        match format {
            OutputFormat::Json => println!("[]"),
            _ => println!("No collections found."),
        }
        return Ok(());
    }

    let mut collections = Vec::new();
    let mut entries = tokio::fs::read_dir(&collections_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        if entry.path().is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Count documents
            let mut doc_count = 0;
            let mut docs = tokio::fs::read_dir(entry.path()).await?;
            while let Some(doc) = docs.next_entry().await? {
                if doc.path().extension().map(|e| e == "md").unwrap_or(false) {
                    doc_count += 1;
                }
            }
            collections.push((name, doc_count));
        }
    }

    match format {
        OutputFormat::Json => {
            let json: Vec<serde_json::Value> = collections
                .iter()
                .map(|(name, count)| serde_json::json!({"name": name, "documents": count}))
                .collect();
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Table => {
            println!("Collections:");
            for (name, count) in &collections {
                println!("  {} ({} documents)", name, count);
            }
        }
        OutputFormat::Minimal => {
            for (name, _) in &collections {
                println!("{}", name);
            }
        }
    }

    Ok(())
}

async fn list_views(path: &PathBuf, format: OutputFormat) -> anyhow::Result<()> {
    let views_path = path.join(".mdby/views");

    if !views_path.exists() {
        match format {
            OutputFormat::Json => println!("[]"),
            _ => println!("No views found."),
        }
        return Ok(());
    }

    let mut views = Vec::new();
    let mut entries = tokio::fs::read_dir(&views_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        if entry.path().extension().map(|e| e == "yaml").unwrap_or(false) {
            let name = entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            views.push(name);
        }
    }

    match format {
        OutputFormat::Json => {
            let json: Vec<serde_json::Value> = views
                .iter()
                .map(|name| serde_json::json!({"name": name}))
                .collect();
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Table => {
            println!("Views:");
            for name in &views {
                println!("  {}", name);
            }
        }
        OutputFormat::Minimal => {
            for name in &views {
                println!("{}", name);
            }
        }
    }

    Ok(())
}
