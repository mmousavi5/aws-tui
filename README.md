cat > README.md << 'EOF'
# AWS Terminal UI (aws-tui)

A terminal-based user interface for interacting with AWS services without leaving your command line.

## Overview

aws-tui provides an intuitive terminal interface for managing AWS resources including S3 buckets, DynamoDB tables, and CloudWatch logs. Built with Rust and the Ratatui framework, it offers a responsive, keyboard-driven experience for cloud practitioners who prefer terminal-based workflows.

## Features

- Multi-service Support:
  - S3: Browse buckets and objects, navigate directories
  - DynamoDB: Query tables, view table data as formatted JSON
  - CloudWatch: Search log groups, view and filter log entries
- Multi-tab Interface: Work with different services or profiles simultaneously
- AWS Profile Switching: Easily switch between profiles from the ~/.aws/config file
- Keyboard Navigation: Intuitive shortcuts for productive workflows
- Rich Data Display: Formatted JSON, syntax highlighting, and filtering

## Architecture

The application follows an event-driven architecture with a clear separation of concerns:
```ascii
+----------------+      +-------------------+      +-----------------+
|   Terminal UI  | ---> |  Event Management | ---> |    Components   |
|   (Ratatui)    |      |                   |      |                 |
+----------------+      +-------------------+      +-----------------+
       ^                         ^                         |
       |                         |                         v
       |                         |                 +-----------------+
       |                         |                 |  AWS Services   |
       |                         |                 |    Clients      |
       |                         |                 +-----------------+
       |                         |                         |
       |                         v                         v
+----------------+      +-------------------+      +-----------------+
|     Render     | <--- |  Application      | <--- |  AWS SDK Rust   |
|                |      |     State         |      |                 |
+----------------+      +-------------------+      +-----------------+

## Code Structure

aws-tui/
├── src/
│   ├── main.rs               # Entry point with initialization
│   ├── app.rs                # Application state and event loop
│   ├── ui.rs                 # UI rendering logic
│   ├── components/           # AWS service components
│   │   ├── aws_base_component.rs  # Shared component behavior
│   │   ├── s3.rs             # S3 browser interface
│   │   ├── dynamodb.rs       # DynamoDB query interface
│   │   ├── cloudwatch.rs     # CloudWatch logs interface
│   │   └── tab.rs            # Tab container logic
│   ├── event_managment/      # Event handling system
│   │   ├── event.rs          # Event types and handlers
│   │   └── mod.rs
│   ├── services/             # AWS SDK integration
│   │   ├── aws/              # Service client implementations
│   │   │   ├── s3_client.rs
│   │   │   ├── dynamodb_client.rs
│   │   │   ├── cloudwatch_client.rs
│   │   │   └── tab_clients.rs  # Client manager
│   │   ├── read_config.rs    # AWS profile configuration
│   │   └── mod.rs
│   └── widgets/              # UI building blocks
│       ├── aws_service_navigator.rs # Navigation widget
│       ├── input_box.rs      # Text input widget
│       ├── popup.rs          # Modal dialog widget
│       └── mod.rs            # Widget trait definitions
└── Cargo.toml                # Dependencies and metadata

## Data Flow

1. User Input: Keyboard events are captured by the terminal
2. Event Processing: Events are routed to appropriate handlers based on current UI state
3. Component Update: Component state updates based on processed events
4. AWS Service Call: Service clients execute AWS API operations when needed
5. State Update: Application state is updated with results from AWS
6. UI Rendering: Terminal UI is redrawn to reflect the current state

## Installation

### Prerequisites

- Rust toolchain (stable, 1.70+)
- AWS credentials configured in ~/.aws/config
- Terminal with UTF-8 support

### Building from Source

# Clone the repository
git clone https://github.com/yourusername/aws-tui.git
cd aws-tui
cargo build --release

# Run the application
./target/release/aws-tui

## Keyboard Shortcuts

| Shortcut         | Action                               |
|------------------|--------------------------------------|
| Ctrl+Q           | Quit application                     |
| Ctrl+T           | Create new tab                       |
| Ctrl+W           | Close current tab                    |
| Ctrl+Tab         | Switch to next tab                   |
| Alt+1            | Focus navigation panel               |
| Alt+2            | Focus results panel                  |
| Alt+3            | Focus input panel                    |
| Enter            | Select item or execute query         |
| Esc              | Close popup or exit filter mode      |
| Ctrl+F or /      | Filter items in navigator            |
| ↑/↓              | Navigate through items               |
| PgUp/PgDn        | Scroll through content               |
| Home/End         | Jump to start/end of list            |

## Profile Management

The application automatically reads profiles from your ~/.aws/config file. You can switch profiles at any time by:

1. Opening the profile popup (Alt+P)
2. Selecting a profile with arrow keys
3. Pressing Enter to activate the selected profile

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
EOF