use anyhow::Result;
use crate::cli::{Cli, Commands};
use crate::obsidian_adapter::ObsidianAdapter;
use crate::todos::handle_todo_command;
use crate::notes::handle_note_command;
use crate::goals::handle_goal_command;
use crate::chat::handle_chat_command;

pub async fn route_command(cli: Cli, adapter: &ObsidianAdapter) -> Result<()> {
    match cli.command {
        Commands::Todo(todo_args) => {
            handle_todo_command(todo_args, adapter).await
        }
        Commands::Note(note_args) => {
            handle_note_command(note_args, adapter).await
        }
        Commands::Goal(goal_args) => {
            handle_goal_command(goal_args, adapter).await
        }
        Commands::Chat(chat_args) => {
            handle_chat_command(chat_args, adapter).await
        }
    }
}
