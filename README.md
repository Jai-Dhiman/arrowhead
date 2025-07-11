# Arrowhead

An AI-powered productivity and workflow automation framework that combines intelligent task management, natural language processing, and extensible tool orchestration.

## Overview

Arrowhead is a Rust-based framework that provides intelligent productivity management through AI integration, workflow automation, and extensive plugin capabilities. It features a comprehensive MCP (Model Context Protocol) client implementation, natural language command processing, and seamless integration with productivity tools like Obsidian, Jira, and calendar systems.

## Core Features

### 🧠 AI-Powered Intelligence
- **Conversational Interface**: Natural language interaction with AI models
- **Context Management**: Intelligent context awareness and memory
- **Proactive Assistance**: AI-driven suggestions and automation
- **User Learning**: Adaptive behavior based on usage patterns

### 📝 Advanced Productivity Management
- **Todo Management**: Create, track, and manage todos with AI assistance
- **Note Management**: Intelligent note creation and organization
- **Goal Management**: Set and track goals with AI-powered insights
- **Workflow Automation**: Customizable workflows with monitoring

### 🔧 Tool Orchestration & Integration
- **MCP Client Framework**: Comprehensive Model Context Protocol implementation
- **Plugin System**: Extensible architecture for custom tools and integrations
- **Multi-Service Integration**: Connect with Obsidian, Jira, calendars, and more
- **Natural Language CLI**: Convert natural language to structured commands

### 🌐 Protocol & API Support
- **MCP Protocol**: Full Model Context Protocol client with version negotiation
- **REST API Integration**: Obsidian Local REST API support
- **WebSocket Support**: Real-time communication capabilities
- **Multiple Transport Layers**: StdIO, TCP, WebSocket, and Process transports

## Prerequisites

- **Rust** (latest stable version)
- **Obsidian** with the [Local REST API plugin](https://github.com/coddingtonbear/obsidian-local-rest-api) installed and enabled (for Obsidian integration)
- **AI API Keys** (optional, for AI-powered features):
  - OpenAI, Anthropic, Google, or other supported AI providers

## Installation

### 🚀 One-Line Installation (Recommended)

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/Jai-Dhiman/arrowhead/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
iwr https://raw.githubusercontent.com/Jai-Dhiman/arrowhead/main/install.ps1 | iex
```

### 📦 Alternative Installation Methods

**From Source:**
```bash
git clone https://github.com/Jai-Dhiman/arrowhead.git
cd arrowhead
cargo build --release
cargo install --path .
```

**Using Cargo (requires Rust):**
```bash
cargo install arrowhead
```

**Download Pre-built Binaries:**
- Visit the [releases page](https://github.com/Jai-Dhiman/arrowhead/releases)
- Download the appropriate binary for your platform
- Extract and add to your PATH

### 🎯 Quick Start

After installation, get started in 2 minutes:

1. **Set up your AI API key** (get a free Gemini key at [aistudio.google.com](https://aistudio.google.com/app/apikey)):
   ```bash
   export GEMINI_API_KEY="your_api_key_here"
   ```

2. **Start the interactive AI assistant**:
   ```bash
   arrowhead
   ```

3. **Try some natural language commands**:
   - "Create a todo to review the quarterly report"
   - "What should I work on today?"
   - "Help me organize my tasks"

## Configuration

### Basic Setup

Arrowhead can be configured through environment variables or configuration files:

```bash
# API Keys (optional)
export OPENAI_API_KEY="your-openai-key"
export ANTHROPIC_API_KEY="your-anthropic-key"
export GOOGLE_API_KEY="your-google-key"

# Obsidian Integration
export OBSIDIAN_API_URL="http://localhost:27123"
```

### Obsidian Integration Setup

1. Install the "Local REST API" community plugin in Obsidian
2. Enable the plugin in Settings → Community Plugins
3. Configure the plugin to run on port 27123 (default)
4. Ensure your vault is open in Obsidian

### MCP Server Configuration

For MCP server integrations, create a configuration file:

```json
{
  "mcp_servers": [
    {
      "name": "file-manager",
      "transport": "stdio",
      "command": "python",
      "args": ["-m", "file_manager_mcp"]
    }
  ]
}
```

## Usage

### Command Line Interface

Arrowhead provides both traditional CLI commands and natural language processing:

#### Traditional Commands

```bash
# Todo Management
arrowhead todo add "Review project proposal" --due-date "2024-02-15" --tags work urgent
arrowhead todo list --status open
arrowhead todo done "review-project-proposal"

# Note Management
arrowhead note create "Meeting Notes" --content "Discussion points..." --tags meeting work
arrowhead note view "meeting-notes"
arrowhead note append "meeting-notes" "Follow-up: Send summary to team"

# Goal Management
arrowhead goal add "Learn Rust" --description "Complete the Rust book" --target-date "2024-06-01"
arrowhead goal list
arrowhead goal update "learn-rust" --status "in-progress"

# Workflow Management
arrowhead workflow create "Daily Standup" --trigger "daily" --actions "collect-updates,send-summary"
arrowhead workflow list
arrowhead workflow run "daily-standup"
```

#### Natural Language Interface

```bash
# Natural language commands
arrowhead nl "Create a todo to review the quarterly report by Friday"
arrowhead nl "Show me all my overdue tasks"
arrowhead nl "Schedule a meeting with the team next Tuesday"

# Conversational AI
arrowhead chat
> "What should I work on today?"
> "Create a workflow for my morning routine"
> "Analyze my productivity patterns this week"
```

### MCP Client Operations

```bash
# Connect to MCP servers
arrowhead mcp connect --server file-manager
arrowhead mcp list-tools
arrowhead mcp call-tool "list_files" --args '{"path": "/documents"}'

# Plugin management
arrowhead plugin install my-custom-plugin
arrowhead plugin list
arrowhead plugin activate my-custom-plugin
```

### Integration Commands

```bash
# Jira Integration
arrowhead jira sync
arrowhead jira create-issue "Bug in user authentication" --project "PROJ"

# Calendar Integration
arrowhead calendar sync
arrowhead calendar create-event "Team Meeting" --time "2024-02-15T10:00:00"

# Obsidian Integration
arrowhead obsidian sync
arrowhead obsidian create-note "Daily Journal" --vault "main"
```

## Architecture

### Core Components

```
arrowhead/
├── src/
│   ├── main.rs                      # Entry point and CLI routing
│   ├── cli.rs                       # Command line interface definitions
│   ├── lib.rs                       # Core library exports
│   │
│   ├── AI & Intelligence/
│   │   ├── ai_conversation.rs       # AI conversation management
│   │   ├── context_manager.rs       # Context and memory management
│   │   ├── conversational_interface.rs # Natural language interface
│   │   ├── intelligent_help.rs      # AI-powered help system
│   │   ├── proactive_assistance.rs  # Proactive AI suggestions
│   │   └── user_learning.rs         # User behavior learning
│   │
│   ├── Natural Language Processing/
│   │   ├── nl_command_parser.rs     # Natural language to command parsing
│   │   └── nl_cli_bridge.rs         # Bridge between NL and CLI
│   │
│   ├── MCP & Protocol Support/
│   │   ├── mcp_client.rs           # MCP client implementation
│   │   └── mcp_api.rs              # MCP API wrapper
│   │
│   ├── Workflow & Orchestration/
│   │   ├── workflow_engine.rs       # Workflow execution engine
│   │   ├── workflow_templates.rs    # Workflow templates
│   │   ├── workflow_ui.rs          # Workflow user interface
│   │   ├── workflow_monitoring.rs   # Workflow monitoring
│   │   ├── workflow_integrations.rs # Integration orchestration
│   │   └── tool_orchestrator.rs     # Tool coordination
│   │
│   ├── Productivity Management/
│   │   ├── todos.rs                # Todo management
│   │   ├── notes.rs                # Note management
│   │   ├── goals.rs                # Goal management
│   │   └── chat.rs                 # Chat interface
│   │
│   ├── External Integrations/
│   │   ├── obsidian_adapter.rs     # Obsidian REST API integration
│   │   ├── jira_adapter.rs         # Jira integration
│   │   └── calendar_adapter.rs     # Calendar integration
│   │
│   └── Utilities/
│       ├── router.rs               # Request routing
│       └── utils.rs                # Utility functions
│
├── docs/                           # Documentation
│   ├── mcp_client_guide.md        # MCP client documentation
│   └── mcp_tool_development_guide.md # Tool development guide
│
└── Cargo.toml                     # Dependencies and project metadata
```

### Key Design Patterns

1. **Modular Architecture**: Each major feature is isolated in its own module
2. **AI-First Design**: Intelligence is integrated throughout the system
3. **Protocol Agnostic**: Support for multiple communication protocols
4. **Extensible Plugin System**: Easy integration of new tools and services
5. **Context-Aware Operations**: AI maintains context across interactions

## Technical Details

### Core Technologies

- **Language**: Rust 2021 Edition
- **Async Runtime**: Tokio with full feature set
- **CLI Framework**: Clap v4 with derive macros
- **HTTP Client**: Reqwest with JSON support
- **Serialization**: Serde ecosystem (JSON, YAML)
- **Error Handling**: anyhow + thiserror for comprehensive error management

### Key Dependencies

```toml
# Core functionality
tokio = { version = "1.35.1", features = ["full"] }
clap = { version = "4.4.8", features = ["derive"] }
serde = { version = "1.0.193", features = ["derive"] }
reqwest = { version = "0.11.23", features = ["json"] }

# AI and ML support
nalgebra = "0.32"           # Vector operations
rayon = "1.8"               # Parallel processing

# Utilities
uuid = { version = "1.6.1", features = ["v4"] }
chrono = { version = "0.4.31", features = ["serde"] }
futures = "0.3.30"
```

### Data Organization

```
vault/
├── Todos/                  # Task management
├── Notes/                  # Knowledge base
├── Goals/                  # Objective tracking
├── Workflows/              # Automation templates
├── Context/                # AI conversation history
└── Integrations/           # External service data
```

## Development Status

**Current Status**: Advanced Development (v0.1.0)

### ✅ Implemented Core Features
- **Complete MCP Client Framework** with protocol negotiation and tool orchestration
- **AI-Powered Conversational Interface** with context management
- **Natural Language Processing** for command parsing and CLI integration
- **Comprehensive Workflow Engine** with templates and monitoring
- **Multi-Service Integration** (Obsidian, Jira, Calendar)
- **Advanced Productivity Management** with AI assistance
- **Extensible Plugin Architecture** with tool development framework

### ✅ Advanced Capabilities
- **Context-Aware AI Operations** with user learning
- **Proactive Assistance System** with intelligent suggestions
- **Tool Orchestration Framework** for complex automations
- **Multi-Protocol Support** (StdIO, TCP, WebSocket, Process)
- **Comprehensive Error Handling** with graceful degradation
- **Feature Flag System** for dynamic capability management

### 🚧 In Progress
- **Enhanced UI Components** for workflow visualization
- **Advanced Analytics** for productivity insights
- **Plugin Marketplace** for community extensions
- **Real-time Collaboration** features

### 📋 Planned Enhancements
- **Web Interface** for remote management
- **Mobile Companion App** for on-the-go access
- **Advanced AI Models** integration (local LLMs)
- **Enterprise Security** features
- **Cloud Synchronization** capabilities

## MCP Integration

Arrowhead provides a comprehensive MCP (Model Context Protocol) client framework with advanced features:

### Key MCP Features

- **Protocol Version Negotiation**: Automatic compatibility handling
- **Multiple Transport Layers**: StdIO, TCP, WebSocket, Process
- **Tool Discovery & Registration**: Dynamic tool management
- **Feature Flag System**: Progressive enhancement support
- **Plugin Architecture**: Extensible tool development

### MCP Client Usage

```rust
use arrowhead::mcp_api::{MCPClientBuilder, tool_args};

// Create MCP client
let mut client = MCPClientBuilder::new()
    .with_stdio_transport()
    .with_client_info("arrowhead", "0.1.0")
    .build()?;

// Connect and discover tools
client.connect().await?;
let tools = client.list_tools().await?;

// Call tools
let result = client.call_tool("process_data", tool_args!(
    "input" => "sample data",
    "format" => "json"
)).await?;
```

For comprehensive MCP documentation, see [docs/mcp_client_guide.md](docs/mcp_client_guide.md).

## Plugin Development

Arrowhead supports extensive plugin development for custom tools and integrations:

### Creating Custom Tools

```rust
use arrowhead::mcp_client::ToolMetadata;

pub struct CustomTool {
    metadata: ToolMetadata,
}

impl CustomTool {
    pub async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        // Tool implementation
    }
}
```

### Plugin Architecture

- **Native Plugins**: Direct Rust integration
- **MCP Server Plugins**: External process integration
- **Configuration Management**: TOML-based configuration
- **Lifecycle Management**: Init, activate, deactivate, shutdown

For detailed plugin development, see [docs/mcp_tool_development_guide.md](docs/mcp_tool_development_guide.md).

## Contributing

Contributions are welcome! This project embraces both traditional CLI development and cutting-edge AI integration.

### Development Setup

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes and add tests
4. Run the full test suite: `cargo test`
5. Submit a pull request

### Code Style & Standards

```bash
# Format code
cargo fmt

# Run linting
cargo clippy

# Run tests
cargo test

# Run integration tests
cargo test --test integration_tests
```

### Areas for Contribution

- **AI Integration**: New AI model providers and capabilities
- **MCP Tools**: Custom tools and server implementations
- **Workflow Templates**: Pre-built automation workflows
- **Service Integrations**: New productivity service connectors
- **Documentation**: Guides, examples, and tutorials

## License

[Add your chosen license here]

## Acknowledgments

- **[Obsidian](https://obsidian.md/)** for the excellent knowledge management platform
- **[Local REST API plugin](https://github.com/coddingtonbear/obsidian-local-rest-api)** for enabling programmatic vault access
- **[Anthropic](https://anthropic.com/)** for the Model Context Protocol specification
- **[OpenAI](https://openai.com/)** for AI capabilities and API standards
- **Rust Community** for the excellent ecosystem of crates and tools 