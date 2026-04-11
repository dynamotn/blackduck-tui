# Black Duck TUI

A Terminal User Interface (TUI) for browsing Black Duck Software Composition Analysis (SCA) data.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)

## Features

- 🔍 **Browse Projects & Versions**: Navigate through your Black Duck projects and their versions
- 📦 **Component Management**: View BOM (Bill of Materials) components with detailed information
- 🛡️ **Security Insights**: Browse vulnerabilities with CVE details and severity scores
- ⚠️ **Policy Violations**: Track policy rule violations across components
- 🔎 **Advanced Filtering**: Filter components by:
  - Policy Status (IN_VIOLATION, NOT_IN_VIOLATION, etc.)
  - Review Status (UNREVIEWED, REVIEWED, etc.)
  - Approval Status (APPROVED, REJECTED, PENDING)
  - Policy Rule Name (server-side filtering)
- 🔍 **Search**: Quick search across component names
- ⌨️ **Keyboard Navigation**: Vim-style keybindings for efficient navigation
- 📊 **Pagination**: Handle large datasets with automatic pagination

## Installation

### Prerequisites

- Rust 1.70 or higher
- Access to a Black Duck server
- Black Duck API token

### Build from Source

```bash
git clone https://github.com/dynamotn/blackduck-tui.git
cd blackduck-tui || exit
cargo build --release
```

The binary will be available at `target/release/blackduck-tui`.

## Configuration

Black Duck TUI looks for configuration in the following locations (in order):

1. `~/.config/blackduck-tui/config.toml` (Linux/macOS)
2. `%APPDATA%\blackduck-tui\config.toml` (Windows)
3. Environment variables

### Configuration File

Create a `config.toml` file:

```toml
[server]
url = "https://your-blackduck-server.com"
token = "your-api-token-here"
accept_invalid_certs = false

[tui]
page_size = 100
```

### Environment Variables

You can also use environment variables (they override config file values):

```bash
export BLACKDUCK_URL="https://your-blackduck-server.com"
export BLACKDUCK_TOKEN="your-api-token-here"
export BLACKDUCK_ACCEPT_INVALID_CERTS="false"
```

## Usage

### Running the Application

```bash
# Using the binary
./target/release/blackduck-tui

# Or with cargo
cargo run --release

# With debug logging
RUST_LOG=debug cargo run --release
```

### Keyboard Shortcuts

#### General Navigation
- `↑`/`k` - Move up
- `↓`/`j` - Move down
- `←`/`h` - Move left panel / Go back
- `→`/`l` - Move right panel / Go forward
- `Tab` - Switch focus between left and right panels
- `Esc` - Go back to previous screen / Close popup
- `q` - Quit application

#### Tabs (when viewing a version)
- `1` - Components tab
- `2` - Vulnerabilities tab
- `3` - Policy Violations tab

#### Filtering & Search
- `f` - Open filter popup
- `/` - Open search input
- `Space` - Toggle filter option (in filter popup)
- `Enter` - Apply filter / Confirm selection

#### Pagination
- `n` - Next page
- `p` - Previous page

### Navigation Flow

1. **Login Screen**: Enter your credentials (if not configured)
2. **Projects List**: Browse and select a project
3. **Versions List**: Select a project version
4. **Version Details**: View components, vulnerabilities, and policy violations
   - Switch between tabs using `1`, `2`, `3`
   - Filter components using `f`
   - Search components using `/`
   - Navigate pages using `n`/`p`

## Logging

Logs are written to:
- Linux/macOS: `~/.local/share/blackduck-tui/blackduck-tui.log`
- Windows: `%LOCALAPPDATA%\blackduck-tui\blackduck-tui.log`

Enable debug logging:
```bash
RUST_LOG=debug ./blackduck-tui
```

## Architecture

The application is structured as follows:

```
src/
├── main.rs           # Entry point, TUI setup
├── app.rs            # Application state management
├── events.rs         # Event handling and async operations
├── ui/               # UI rendering
│   └── mod.rs
├── api/              # Black Duck API client
│   ├── client.rs     # HTTP client implementation
│   ├── types.rs      # API response types
│   └── error.rs      # Error types
└── config.rs         # Configuration management
```

### Key Technologies

- **[ratatui](https://github.com/ratatui-org/ratatui)**: Terminal UI framework
- **[crossterm](https://github.com/crossterm-rs/crossterm)**: Cross-platform terminal manipulation
- **[tokio](https://tokio.rs/)**: Async runtime
- **[reqwest](https://github.com/seanmonstar/reqwest)**: HTTP client
- **[serde](https://serde.rs/)**: Serialization/deserialization

## Development

### Running Tests

```bash
cargo test
```

### Running Clippy

```bash
cargo clippy --all-targets --locked -- -D warnings
```

### Code Formatting

```bash
cargo fmt
```

## API Integration

The application uses the Black Duck REST API with the following key endpoints:

- `/api/tokens/authenticate` - Authentication
- `/api/projects` - List projects
- `/api/projects/{id}/versions` - List versions
- `/api/versions/{id}/components` - List BOM components
- `/api/versions/{id}/vulnerable-bom-components` - List vulnerabilities
- `/api/versions/{id}/policy-status` - List policy violations
- `/api/versions/{id}/components-filters` - Get available filter options

All filtering is performed server-side for optimal performance.

## Troubleshooting

### Authentication Issues

If you see "Failed to authenticate":
1. Verify your Black Duck server URL is correct
2. Check that your API token is valid
3. Ensure network connectivity to the Black Duck server

### Certificate Issues

If you're using a self-signed certificate:
```toml
[server]
accept_invalid_certs = true
```

Or set environment variable:
```bash
export BLACKDUCK_ACCEPT_INVALID_CERTS="true"
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Guidelines

1. Follow Rust best practices and idioms
2. Maintain test coverage
3. Ensure clippy passes with no warnings
4. Update documentation as needed

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with [ratatui](https://github.com/ratatui-org/ratatui)
- Inspired by terminal UI tools like lazygit and k9s

## Roadmap

- [ ] Export components/vulnerabilities to CSV/JSON
- [ ] Component details view
- [ ] Vulnerability remediation guidance
- [ ] Custom filter presets
- [ ] Multi-version comparison
- [ ] Dark/light theme support
