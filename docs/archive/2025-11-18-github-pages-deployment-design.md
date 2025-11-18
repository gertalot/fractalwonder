# GitHub Pages Automated Deployment Design

**Date:** 2025-11-18
**Status:** Approved
**Platform:** GitHub Pages with coi-serviceworker
**Deployment:** Auto-deploy on every push to main

## Overview

Automatically deploy Fractal Wonder to GitHub Pages when pushing to the main branch. Uses coi-serviceworker to enable SharedArrayBuffer support (required for Web Workers with shared memory).

### Deployment Flow

1. Push to `main` branch triggers GitHub Actions workflow
2. Build process:
   - Install Rust toolchain with wasm32 target
   - Install Trunk build tool
   - Run `trunk build --release` → generates `dist/` directory
   - coi-serviceworker files included from source repository
3. Deploy `dist/` contents to `gh-pages` branch
4. GitHub Pages serves site from `gh-pages` branch
5. Site live at `https://gertalot.github.io/fractalwonder`

### Build Performance

- **First run:** 5-8 minutes (downloading dependencies)
- **Subsequent runs:** 2-3 minutes (with caching)
- **Cache strategy:** Cargo registry, git dependencies, and build artifacts

## GitHub Actions Workflow

### Trigger Configuration

```yaml
on:
  push:
    branches: [main]
```

Runs on every push to main branch. No manual approval required.

### Job Structure

**Runner:** `ubuntu-latest`

**Permissions:**
- `contents: write` - Required to push to gh-pages branch

### Workflow Steps

1. **Checkout repository**
   - Tool: `actions/checkout@v4`
   - Fetches source code from main branch

2. **Setup Rust toolchain**
   - Tool: `dtolnay/rust-toolchain@stable`
   - Installs stable Rust compiler
   - Adds `wasm32-unknown-unknown` target for WebAssembly compilation
   - Configures rustup environment

3. **Cache Rust dependencies**
   - Tool: `Swatinem/rust-cache@v2`
   - Caches:
     - `~/.cargo/registry` - crate registry index and downloads
     - `~/.cargo/git` - git dependencies
     - `target/` - compiled build artifacts
   - Cache key: Hash of `Cargo.lock`
   - Significantly reduces build time on subsequent runs

4. **Install Trunk**
   - Command: `cargo install trunk`
   - Cached after first installation
   - Required for building Leptos WASM applications

5. **Build release version**
   - Command: `trunk build --release`
   - Outputs to `dist/` directory
   - Includes:
     - Compiled WASM modules
     - JavaScript glue code
     - HTML entry point
     - Tailwind CSS (processed by Trunk)
     - Static assets from `static/` directory

6. **Deploy to GitHub Pages**
   - Tool: `peaceiris/actions-gh-pages@v4`
   - Pushes `dist/` contents to `gh-pages` branch
   - Auto-creates `gh-pages` branch if it doesn't exist
   - Force-push (only latest build kept, no history)
   - Uses `GITHUB_TOKEN` for authentication (automatically provided)

### Environment Requirements

- **Node.js:** Pre-installed on ubuntu-latest (required for Tailwind CSS via Trunk)
- **Git:** Pre-installed (required for checkout and deployment)
- **Network access:** Required for downloading Rust crates and tools

## coi-serviceworker Integration

### Purpose

GitHub Pages doesn't support custom HTTP headers. SharedArrayBuffer requires these headers for security:
- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`

coi-serviceworker is a service worker that intercepts HTTP responses and adds these headers client-side.

### Implementation

**Files to add:**

1. `static/coi-serviceworker.js` (~2KB)
   - Source: https://github.com/gzuidhof/coi-serviceworker
   - Main service worker file
   - Automatically detects and registers itself

2. `static/coi-serviceworker.min.js`
   - Minified version (referenced by main file)
   - Same source repository

Both files placed in `static/` directory. Trunk copies `static/` to `dist/` during build.

**HTML modification:**

Add to `index.html` in `<head>` section (before WASM loading):

```html
<script src="coi-serviceworker.js"></script>
```

### How It Works

1. Browser loads `index.html`
2. Service worker script executes
3. Registers service worker if supported
4. Service worker intercepts all network requests
5. Adds COOP/COEP headers to responses
6. First visit: Page reloads once to activate service worker
7. Subsequent visits: Service worker already active, no reload
8. SharedArrayBuffer becomes available for Web Workers

### Local Development

The service worker doesn't interfere with local development:
- `trunk serve` already provides COOP/COEP headers (defined in `Trunk.toml`)
- Service worker detects headers are present and remains inactive
- No performance impact during development

### Testing Service Worker

To test the service worker locally:

```bash
trunk build --release
cd dist
python3 -m http.server 8080
```

Visit `http://localhost:8080` and check browser console for:
- Service worker registration messages
- "SharedArrayBuffer is defined" confirmation

## GitHub Pages Configuration

### One-Time Setup

After first workflow run:

1. Navigate to repository settings: `https://github.com/gertalot/fractalwonder/settings/pages`
2. Under "Build and deployment":
   - **Source:** Deploy from a branch
   - **Branch:** `gh-pages`
   - **Directory:** `/ (root)`
3. Save changes

The workflow creates the `gh-pages` branch automatically, but GitHub needs to be told to serve from it.

### Site URL

**Default:** `https://gertalot.github.io/fractalwonder`

### Custom Domain (Optional)

To use a custom domain:

1. Add `CNAME` file to `static/` directory containing domain name
2. Configure DNS with domain provider (A or CNAME records)
3. Update GitHub Pages settings to use custom domain
4. GitHub automatically handles HTTPS via Let's Encrypt

## Branch Strategy

### Source Branch: `main`

- Contains source code (Rust, configuration files, etc.)
- You push code here
- Triggers deployment workflow

### Deployment Branch: `gh-pages`

- Auto-generated by workflow
- Contains built artifacts only (HTML, WASM, JS, CSS)
- Never manually edited
- Force-pushed on each deployment (no history preserved)
- Served by GitHub Pages

## Monitoring and Operations

### Deployment Monitoring

**GitHub Actions tab:**
- View workflow runs and status
- See build logs for debugging
- Track build timing and performance

**Deployments section (repo sidebar):**
- Track deployment history
- See active deployment
- Monitor deployment status

### Rollback Process

If a deployment introduces bugs:

```bash
# Revert the problematic commit on main
git revert <commit-hash>
git push origin main

# Workflow automatically runs and deploys the reverted version
```

Deployment completes in 2-3 minutes (using cached dependencies).

### Common Issues

**Build fails:**
- Check Actions tab for error logs
- Verify `Cargo.toml` and `Cargo.lock` are committed
- Ensure all dependencies are accessible from CI environment

**Site not updating:**
- Check workflow completed successfully
- Clear browser cache
- Service worker may need refresh (hard reload: Cmd+Shift+R)

**SharedArrayBuffer not working:**
- Check browser console for service worker errors
- Verify `coi-serviceworker.js` files are present in deployed site
- Confirm service worker registration succeeded
- Some browsers block service workers in private/incognito mode

## Security Considerations

### GitHub Token

- `GITHUB_TOKEN` automatically provided by GitHub Actions
- Scoped to repository only
- No manual token configuration required
- Permissions limited to pushing to branches

### Service Worker

- Open source and auditable
- No external network calls
- No data collection or tracking
- Only modifies response headers locally
- Widely used in production (Unity WebGL, Emscripten, etc.)

## Future Enhancements

Possible improvements not included in initial implementation:

- **Preview deployments:** Deploy PRs to temporary URLs for testing
- **Build artifacts:** Save WASM files for download/analysis
- **Performance metrics:** Track bundle sizes over time
- **Deployment notifications:** Slack/Discord webhooks on deployment
- **Multiple environments:** Staging vs production deployments

These can be added incrementally as needed.
