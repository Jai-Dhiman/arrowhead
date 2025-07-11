use clap::{Parser, Subcommand, Args};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize)]
pub enum Commands {
    /// Manage todos
    Todo(TodoArgs),
    /// Manage goals
    Goal(GoalArgs),
    /// Manage notes
    Note(NoteArgs),
    /// Manage configuration (API keys, settings, etc.)
    Config(ConfigArgs),
}

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct TodoArgs {
    #[clap(subcommand)]
    pub action: TodoAction,
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize)]
pub enum TodoAction {
    /// Add a new todo
    Add {
        description: String,
        #[clap(short, long)]
        due_date: Option<String>,
        #[clap(short, long, value_parser)]
        tags: Vec<String>,
    },
    /// List all todos
    List {
        #[clap(short, long)]
        status: Option<String>, // e.g., "open", "done"
    },
    /// Mark a todo as done
    Done {
        id: String, // Or some identifier
    },
    /// View a specific todo
    View {
        id: String,
    }
}

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct GoalArgs {
    #[clap(subcommand)]
    pub action: GoalAction,
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize)]
pub enum GoalAction {
    /// Add a new goal
    Add {
        title: String,
        #[clap(short, long)]
        description: Option<String>,
        #[clap(short, long)]
        target_date: Option<String>,
        #[clap(short, long, value_parser)]
        tags: Vec<String>,
    },
    /// List all goals
    List {
        #[clap(short, long)]
        status: Option<String>, // e.g., "active", "achieved", "on-hold"
    },
    /// Update an existing goal
    Update {
        id: String,
        #[clap(short, long)]
        title: Option<String>,
        #[clap(short, long)]
        description: Option<String>,
        #[clap(short, long)]
        status: Option<String>,
        #[clap(short, long)]
        target_date: Option<String>,
    },
    /// View a specific goal
    View {
        id: String,
    }
}

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct NoteArgs {
    #[clap(subcommand)]
    pub action: NoteAction,
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize)]
pub enum NoteAction {
    /// Create a new note
    Create {
        title: String,
        #[clap(short, long)]
        content: Option<String>,
        #[clap(short, long, value_parser)]
        tags: Vec<String>,
    },
    /// List all notes
    List {
        #[clap(short, long, value_parser)]
        tags: Vec<String>,
    },
    /// View a specific note
    View {
        /// The name or ID of the note (e.g., "capture-prd")
        name_or_id: String,
    },
    /// Append content to an existing note
    Append {
        name_or_id: String,
        content: String,
    },
    /// Edit an existing note (could open in $EDITOR)
    Edit {
        name_or_id: String,
    }
}


#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct ConfigArgs {
    /// Create sample configuration file
    #[clap(long)]
    pub init: bool,
    /// Show current configuration
    #[clap(long)]
    pub show: bool,
    /// Set configuration values (e.g., gemini.api_key, openai.api_key, provider)
    #[clap(long)]
    pub set: Option<String>,
    /// Value to set (used with --set)
    #[clap(long)]
    pub value: Option<String>,
}

// Example usage (will be in main.rs)
// fn main() {
//     let cli = Cli::parse();
//     println!("{:?}", cli);
// }
