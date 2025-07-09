use arrowhead::cli::Cli;
use arrowhead::obsidian_adapter::ObsidianAdapter;
use arrowhead::router::route_command;
use clap::Parser;

#[tokio::main]
async fn main() {
    // Parse CLI arguments
    let cli_args = Cli::parse();

    // Initialize Obsidian Adapter
    // The base URL for the MCP server can be configured here,
    // potentially from an environment variable or a config file in the future.
    let adapter = ObsidianAdapter::new(None); // Uses default MCP_SERVER_URL

    // Route command to appropriate module
    if let Err(e) = route_command(cli_args, &adapter).await {
        eprintln!("Error: {:?}", e);
        // Consider more user-friendly error reporting here
        // For example, distinguishing between client errors and internal errors.
        std::process::exit(1);
    }
}
