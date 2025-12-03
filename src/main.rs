//! MDBY CLI - Markdown Database

use clap::{Parser, Subcommand};
use mdby::{Database, QueryResult};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mdby")]
#[command(about = "A markdown-based git-backed database", long_about = None)]
struct Cli {
    /// Database directory (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    database: PathBuf,

    #[command(subcommand)]
    command: Commands,
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
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init_database(&cli.database).await,
        Commands::Query { query } => execute_query(&cli.database, &query).await,
        Commands::Repl => run_repl(&cli.database).await,
        Commands::Regenerate => regenerate_views(&cli.database).await,
        Commands::Sync { remote } => sync_database(&cli.database, &remote).await,
        Commands::Status => show_status(&cli.database).await,
        Commands::Collections => list_collections(&cli.database).await,
        Commands::Views => list_views(&cli.database).await,
    }
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

async fn execute_query(path: &PathBuf, query: &str) -> anyhow::Result<()> {
    let mut db = Database::open(path).await?;
    let result = db.execute(query).await?;

    match result {
        QueryResult::Documents(docs) => {
            if docs.is_empty() {
                println!("No documents found.");
            } else {
                for doc in docs {
                    println!("--- {} ---", doc.id);
                    for (key, value) in &doc.fields {
                        println!("  {}: {:?}", key, value);
                    }
                    if !doc.body.is_empty() {
                        println!("  [body]: {} chars", doc.body.len());
                    }
                    println!();
                }
            }
        }
        QueryResult::Affected(count) => {
            println!("{} document(s) affected.", count);
        }
        QueryResult::CollectionCreated(name) => {
            println!("Collection '{}' created.", name);
        }
        QueryResult::ViewCreated(name) => {
            println!("View '{}' created.", name);
        }
    }

    Ok(())
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
                    if docs.is_empty() {
                        println!("(0 rows)");
                    } else {
                        for doc in &docs {
                            println!("--- {} ---", doc.id);
                            for (key, value) in &doc.fields {
                                println!("  {}: {:?}", key, value);
                            }
                        }
                        println!("({} row(s))", docs.len());
                    }
                }
                QueryResult::Affected(n) => println!("({} row(s) affected)", n),
                QueryResult::CollectionCreated(name) => println!("Collection '{}' created", name),
                QueryResult::ViewCreated(name) => println!("View '{}' created", name),
            },
            Err(e) => {
                println!("Error: {}", e);
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

async fn list_collections(path: &PathBuf) -> anyhow::Result<()> {
    let collections_path = path.join("collections");

    if !collections_path.exists() {
        println!("No collections found.");
        return Ok(());
    }

    println!("Collections:");
    let mut entries = tokio::fs::read_dir(&collections_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.path().is_dir() {
            let name = entry.file_name();
            // Count documents
            let mut doc_count = 0;
            let mut docs = tokio::fs::read_dir(entry.path()).await?;
            while let Some(doc) = docs.next_entry().await? {
                if doc.path().extension().map(|e| e == "md").unwrap_or(false) {
                    doc_count += 1;
                }
            }
            println!("  {} ({} documents)", name.to_string_lossy(), doc_count);
        }
    }

    Ok(())
}

async fn list_views(path: &PathBuf) -> anyhow::Result<()> {
    let views_path = path.join(".mdby/views");

    if !views_path.exists() {
        println!("No views found.");
        return Ok(());
    }

    println!("Views:");
    let mut entries = tokio::fs::read_dir(&views_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.path().extension().map(|e| e == "yaml").unwrap_or(false) {
            let name = entry.path().file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            println!("  {}", name);
        }
    }

    Ok(())
}
