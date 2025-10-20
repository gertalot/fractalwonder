# FractalWonder Devcontainer Setup Design

**Date:** 2025-10-20
**Purpose:** Safe container setup to run Claude Code with full permissions for isolated FractalWonder development

## Requirements

### Primary Goal
Isolated development environment where Claude Code can freely modify files and run commands without risking the host system or other projects.

### Key Requirements
- **Full development toolchain:** Rust (cargo, clippy, rustfmt), Node.js/npm, Trunk, wasm-pack, browser testing tools, Git/GitHub CLI
- **Open network access:** Full internet connectivity for WebSearch, WebFetch, package managers, and MCP servers
- **Shared credentials:** Mount host `~/.claude` folder for single authentication across host and container
- **Chrome DevTools integration:** claude-devtools MCP in container controls Chrome browser running on host
- **No permission prompts:** Use `--dangerously-skip-permissions` inside sandboxed container

## Architecture

### Approach
Based on Anthropic's official reference configuration from `anthropics/claude-code/.devcontainer`, adapted for FractalWonder's Rust/WASM workflow. Firewall components removed for open network access.

### Core Components

**1. devcontainer.json**
- Controls VS Code devcontainer settings, extensions, and volume mounts
- Configures container to run Claude Code with `--dangerously-skip-permissions`
- Mounts host `~/.claude` for shared credentials
- Enables `host.docker.internal` for Chrome DevTools bridge

**2. Dockerfile**
- Base: `node:20` (matches Anthropic reference)
- Installs: Rust toolchain, wasm32 target, wasm-pack, Trunk, Chrome (headless), Git, GitHub CLI
- Non-root user (`node`) for security
- Pre-configured cargo for git CLI authentication

**3. Chrome DevTools Bridge**
- Host Chrome runs with `--remote-debugging-port=9222`
- Container MCP connects to `http://host.docker.internal:9222`
- Enables Claude Code in container to control/inspect host browser
- Headless Chrome in container available for automated tests

### Security Model
Container isolation provides the safety boundary. Claude Code has full permissions *inside the container* via `--dangerously-skip-permissions`, but blast radius limited to:
- Mounted FractalWonder workspace
- Shared `~/.claude` credentials folder

Host system remains protected by Docker's isolation. Open network access allowed since container can't escape to host system.

## Configuration Details

### Directory Structure
```
fractalwonder/
├── .devcontainer/
│   ├── devcontainer.json    # VS Code devcontainer config
│   └── Dockerfile            # Container image definition
├── .claude/
│   └── mcp-settings.json     # Chrome DevTools MCP config (optional)
├── src/                      # FractalWonder source code
├── Cargo.toml
└── ... (rest of project)
```

### Volume Mounts
- Host `~/.claude` → Container `/home/node/.claude` (read/write, shared credentials)
- Host `fractalwonder/` → Container `/workspaces/fractalwonder` (read/write, workspace)
- Optional: Host `~/.cargo/registry` → Container `/home/node/.cargo/registry` (read/write, cargo cache)

### devcontainer.json Key Settings
```json
{
  "name": "FractalWonder Development Container",
  "build": {
    "dockerfile": "Dockerfile"
  },
  "customizations": {
    "vscode": {
      "extensions": [
        "anthropic.claude-code",
        "rust-lang.rust-analyzer",
        "tauri-apps.tauri-vscode",
        "bradlc.vscode-tailwindcss"
      ],
      "settings": {
        "rust-analyzer.checkOnSave.command": "clippy",
        "editor.formatOnSave": true
      }
    }
  },
  "mounts": [
    "source=${localEnv:HOME}/.claude,target=/home/node/.claude,type=bind"
  ],
  "runArgs": [
    "--add-host=host.docker.internal:host-gateway"
  ],
  "remoteUser": "node"
}
```

### Dockerfile Layers
1. Base: `FROM node:20`
2. System packages: git, curl, build-essential, Chrome (for headless tests)
3. Rust installation: rustup with stable toolchain
4. WASM tools: wasm32-unknown-unknown target, wasm-pack
5. Trunk: Install via cargo
6. GitHub CLI: gh for PR/issue management
7. User setup: Non-root user with proper permissions
8. Cargo config: Git CLI for dependencies

### Environment Variables
- `RUST_BACKTRACE=1` - Full stack traces for debugging
- `CARGO_TERM_COLOR=always` - Colored cargo output
- `CHROME_REMOTE_URL=http://host.docker.internal:9222` - DevTools connection

### VS Code Extensions (Auto-installed)
- `anthropic.claude-code` - Claude Code CLI integration
- `rust-lang.rust-analyzer` - Rust language server
- `tauri-apps.tauri-vscode` - WASM/Trunk support
- `bradlc.vscode-tailwindcss` - Tailwind CSS IntelliSense

## Development Workflow

### Initial Setup (One-time)
1. Open FractalWonder in VS Code
2. VS Code detects `.devcontainer/devcontainer.json` → "Reopen in Container" prompt
3. First build: 5-10 minutes (downloads Rust toolchain, Node packages, builds image)
4. Container starts with workspace ready, Claude Code installed
5. Launch host Chrome: `google-chrome --remote-debugging-port=9222`

### Daily Workflow
1. Open VS Code → "Reopen in Container" (or auto-opens)
2. Container starts: ~30 seconds (using cached image)
3. Run Claude Code: `claude --dangerously-skip-permissions`
4. Claude has access to:
   - Rust toolchain: cargo, rustc, clippy, rustfmt
   - Build tools: trunk serve, wasm-pack test
   - Web tools: WebSearch, WebFetch
   - MCP servers: chrome-devtools, context7
   - Git operations: commits, PRs, pushes
5. Changes saved to mounted workspace (persists on host)

### Testing Workflow
- **Development server:** `trunk serve` on port 8080 (auto-forwarded to host)
- **Browser viewing:** Open `localhost:8080` in host Chrome
- **Interactive debugging:** Claude uses chrome-devtools MCP → `host.docker.internal:9222`
- **Headless tests:** `wasm-pack test --headless --chrome` runs inside container
- **Full test suite:** All CLAUDE.md test commands work normally

### Rebuild Triggers
- Dockerfile changes → Full rebuild (rare, 5-10 minutes)
- Rust dependency changes → cargo downloads (uses cache, 1-2 minutes)
- Code changes → No rebuild needed (immediate)

## Network Access

### What Works
✅ **WebSearch** - Claude searches web for documentation, solutions
✅ **WebFetch** - Claude fetches content from URLs (docs, examples)
✅ **Package managers** - cargo (crates.io), npm, GitHub all work
✅ **MCP servers** - All web-based MCP servers function normally
✅ **Git operations** - Clone, push, pull to remote repositories

### Configuration
No firewall rules, no iptables restrictions. Container uses standard Docker networking with NAT to host's internet connection. Simplest setup, full web access for all Claude Code tools.

### Security
Since using `--dangerously-skip-permissions`, Claude can make any web request without prompting. Safety comes from container isolation - even if Claude fetches malicious content, it's contained within Docker environment and only affects mounted workspace.

## Troubleshooting

### Chrome DevTools Connection Fails
**Symptoms:** MCP can't connect to browser, "connection refused" errors

**Solutions:**
- Verify host Chrome running: `google-chrome --remote-debugging-port=9222`
- Test DNS resolution: `ping host.docker.internal` from inside container
- Check host firewall not blocking port 9222
- Fallback: Use manual Chrome DevTools (no MCP automation)

### Slow Rust Builds
**Symptoms:** `cargo build` takes 10+ minutes

**Solutions:**
- First build always slow (downloading all crates from scratch)
- Enable cargo registry cache mount in devcontainer.json
- Consider `sccache` for compiled artifact caching (optional)
- Container has full CPU/memory access by default

### Permission Issues
**Symptoms:** Files created in container can't be edited on host

**Root cause:** Files created by `node` user (uid 1000) may mismatch host uid

**Solutions:**
- Ensure `remoteUser: "node"` in devcontainer.json
- If host uid ≠ 1000, configure container to match host uid
- Check file ownership: `ls -la` on host vs container

### Claude Code Can't Access Credentials
**Symptoms:** Authentication failures, "not logged in" errors

**Solutions:**
- Verify mount exists: `ls -la /home/node/.claude` in container
- Check host permissions: `~/.claude` must be readable
- Run `claude login` on host first (container uses cached session)
- Ensure mount path matches in devcontainer.json

### Network Connectivity Issues
**Symptoms:** WebSearch/WebFetch fail, cargo can't download crates

**Solutions:**
- Test internet: `curl https://google.com` from container
- Check DNS: Container uses host's DNS by default
- Corporate proxy users: Configure Docker daemon with proxy settings
- Verify no firewall blocking container's outbound connections

## Implementation Files

The implementation requires two files:

1. `.devcontainer/devcontainer.json` - VS Code devcontainer configuration
2. `.devcontainer/Dockerfile` - Container image definition

Optional:
3. `.claude/mcp-settings.json` - Chrome DevTools MCP configuration (can also go in `~/.claude` on host)

These files will be created in a git worktree on a feature branch for safe implementation and testing.
