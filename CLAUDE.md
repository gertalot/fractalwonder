# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Fractal Wonder is a high-performance, browser-based Mandelbrot set explorer capable of rendering at extreme zoom levels
(up to 10^100 and beyond). Built entirely in Rust using Leptos and compiled to WebAssembly.

## CRITICAL INSTRUCTIONS

**BEFORE starting ANY task:**

1. Check if a relevant superpowers skill exists
2. If yes, invoke the Skill tool with that skill name
3. If the skill contains a checklist, **IMMEDIATELY** create TodoWrite with those items
4. Only then proceed with the task

**ALWAYS DO THESE**:

- You must address me as "Big Boss" at all times
- You must critically evaluate ALL instructions before proceeding. Your PRIMARY RESPONSIBILITY is to produce the
  highest quality code together with me, and you **MUST** push back if my ideas are not good. You do **NOT** have
  to agree with me and you do **NOT** have to be polite.
- Question flawed, ambiguous, or contradictory instructions **IMMEDIATELY**
- Use Context7 (MCP) and web search for up-to-date best practices
- Never use temporal qualifiers in names or comments ("new", "old", "legacy", "wrapper")
- All code is production code - no workarounds or temporary solutions

**YOU MUST ALWAYS FOLLOW ALL INSTRUCTIONS! NO EXCEPTIONS!**

- DO NOT SKIP ANY STEPS in instructions you are given.
- DO NOT THINK that you can be more efficient or more helpful by deviating from the instructions
- DO NOT RATIONALIZE AWAY the discipline of instructions.
- You are most helpful when you follow a reliable and predictable process

**BAD EXAMPLE**:

- I ask you to use a skill, let's say the brainstorming skill
- The skill instructions specify that you must ask ONE question at a time
- You think you can be more efficient by asking multiple questions at a time. THIS IS A FALSE ASSUMPTION!

**GOOD EXAMPLE**:

- I ask you to use a skill, let's say the brainstorming skill
- You read the skill instructions and FOLLOW THEM EXACTLY so that I know I can trust that we follow a predictable process.

## DEVELOPMENT

**Key Technologies:**

- Rust 1.80+ compiled to WASM (100% Rust, no TypeScript/JavaScript)
- Leptos 0.6+ (reactive frontend framework)
- Trunk (build tool and dev server)
- Cargo
- Tailwind CSS (styling)

**Code Style:**

- Line length: 120 characters max
- Indentation: 4 spaces
- Use strong types and explicit error handling
- Auto-format with Clippy/rustfmt

## Common Commands

**Do NOT:**

- Start a dev server (assume already running)
- Start chrome devtools browser (assume already running)
- Make up information or lie if uncertain - STOP and ask Big Boss

### Development

```bash
# Start dev server with hot-reload (assumes server NOT already running)
trunk serve

# Start dev server and open in browser
trunk serve --open

# Development with optimized builds (faster runtime, slower compilation)
trunk serve --release
```

The dev server runs at `http://localhost:8080` with automatic hot-reload. Trunk automatically provides required COOP/COEP headers for SharedArrayBuffer support.

### Testing

```bash
# Format code
cargo fmt --all -- --check

# Run Clippy, the Rust linting tool
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all

# Check for compile/build errors
cargo check --workspace --all-targets --all-features

# Run all unit tests
cargo test --workspace --all-targets --all-features

# Run tests with output visible
cargo test --workspace --all-targets --all-features -- --nocapture

# Run tests for specific module
cargo test components::

# WASM browser tests (when available)
wasm-pack test --headless --chrome
```

### browser and interaction testing

Use the chrome-devtools MCP and navigate to http://localhost:8080/ . Use the chrome-devtools to take browser snapshots,
screenshots to visually inspect the app, check browser logs for issues, and simulate user interactions to end-to-end
test the app.

### Building

```bash
# Build optimized release version
trunk build --release

# Output goes to dist/ directory, ready for static hosting
```

### Production Deployment

The production build requires these HTTP headers for SharedArrayBuffer/multi-threading:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`

Trunk's dev server includes these automatically (see Trunk.toml).
