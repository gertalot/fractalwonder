# GitHub Pages Automated Deployment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Set up automated deployment to GitHub Pages on every push to main branch with SharedArrayBuffer support via coi-serviceworker.

**Architecture:** GitHub Actions workflow builds Rust/WASM app with Trunk, deploys to gh-pages branch. coi-serviceworker enables SharedArrayBuffer on GitHub Pages by adding COOP/COEP headers client-side.

**Tech Stack:** GitHub Actions, Trunk, coi-serviceworker, GitHub Pages

---

## Task 1: Add coi-serviceworker Files

**Files:**
- Create: `static/coi-serviceworker.js`
- Create: `static/coi-serviceworker.min.js`

**Step 1: Create static directory if needed**

Run:
```bash
ls -la static/ 2>/dev/null || mkdir -p static
```

Expected: Directory exists

**Step 2: Download coi-serviceworker.js**

Run:
```bash
curl -o static/coi-serviceworker.js https://raw.githubusercontent.com/gzuidhof/coi-serviceworker/main/coi-serviceworker.js
```

Expected: File downloaded (~2KB)

**Step 3: Download coi-serviceworker.min.js**

Run:
```bash
curl -o static/coi-serviceworker.min.js https://raw.githubusercontent.com/gzuidhof/coi-serviceworker/main/coi-serviceworker.min.js
```

Expected: File downloaded (~1KB)

**Step 4: Verify files**

Run:
```bash
ls -lh static/coi-serviceworker*.js
wc -l static/coi-serviceworker.js
```

Expected: Two files present, main file ~50-100 lines

**Step 5: Commit**

Run:
```bash
git add static/coi-serviceworker.js static/coi-serviceworker.min.js
git commit -m "feat: add coi-serviceworker for SharedArrayBuffer support

Enables SharedArrayBuffer on GitHub Pages by adding COOP/COEP headers
via service worker. Required for Web Workers with shared memory.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit created with 2 files

---

## Task 2: Modify index.html for Service Worker

**Files:**
- Modify: `index.html`

**Step 1: Read current index.html**

Run:
```bash
cat index.html
```

Expected: HTML file with `<head>` section

**Step 2: Add service worker script**

Add this line in the `<head>` section, before any other scripts:

```html
<script src="coi-serviceworker.js"></script>
```

Location: After `<meta>` tags, before any `<link>` or other `<script>` tags

**Step 3: Verify modification**

Run:
```bash
grep -n "coi-serviceworker" index.html
```

Expected: Shows line number with script tag

**Step 4: Commit**

Run:
```bash
git add index.html
git commit -m "feat: register coi-serviceworker in index.html

Loads service worker to enable cross-origin isolation for
SharedArrayBuffer support on GitHub Pages.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit created

---

## Task 3: Create GitHub Actions Workflow

**Files:**
- Create: `.github/workflows/deploy.yml`

**Step 1: Create workflows directory**

Run:
```bash
mkdir -p .github/workflows
```

Expected: Directory created

**Step 2: Create workflow file**

Create `.github/workflows/deploy.yml` with this content:

```yaml
name: Deploy to GitHub Pages

on:
  push:
    branches: [main]

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: write

    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Setup Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Cache Rust dependencies
        uses: Swatinem/rust-cache@v2
        with:
          cache-on-failure: true

      - name: Install Trunk
        run: cargo install trunk --locked

      - name: Build release
        run: trunk build --release

      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v4
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./dist
          force_orphan: true
```

**Step 3: Verify workflow syntax**

Run:
```bash
cat .github/workflows/deploy.yml | head -20
ls -la .github/workflows/
```

Expected: File exists, YAML syntax looks correct

**Step 4: Commit**

Run:
```bash
git add .github/workflows/deploy.yml
git commit -m "feat: add GitHub Actions workflow for automated deployment

Workflow configuration:
- Triggers on push to main branch
- Builds Rust/WASM app with Trunk
- Caches dependencies for faster builds (2-3 min)
- Deploys to gh-pages branch automatically

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit created

---

## Task 4: Verify Local Build

**Files:**
- N/A (verification only)

**Step 1: Run production build**

Run:
```bash
trunk build --release
```

Expected: Build succeeds, creates `dist/` directory

**Step 2: Verify service worker files in dist**

Run:
```bash
ls -la dist/coi-serviceworker*.js
```

Expected: Both coi-serviceworker files present in dist/

**Step 3: Verify index.html has service worker**

Run:
```bash
grep "coi-serviceworker" dist/index.html
```

Expected: Script tag present in built HTML

**Step 4: Test with local server (optional)**

Run:
```bash
cd dist
python3 -m http.server 8080 &
SERVER_PID=$!
sleep 2
echo "Visit http://localhost:8080 and check browser console for service worker registration"
echo "Press Enter when done testing..."
read
kill $SERVER_PID
cd ..
```

Expected: Service worker registers in browser console

**Step 5: Clean up (if ran local server)**

Run:
```bash
rm -rf dist/
```

Expected: Clean working directory

---

## Task 5: Push and Configure GitHub Pages

**Files:**
- N/A (GitHub configuration)

**Step 1: Verify all commits are ready**

Run:
```bash
git log --oneline -5
git status
```

Expected: 3 commits visible, clean working tree

**Step 2: Push to main branch**

Run:
```bash
git push origin feature/deploy-on-pages
```

Expected: Branch pushed successfully

**Step 3: Create pull request**

Run:
```bash
gh pr create --title "feat: automated GitHub Pages deployment" --body "$(cat <<'EOF'
## Summary

Automated deployment to GitHub Pages on every push to main branch.

## Changes

- Add coi-serviceworker for SharedArrayBuffer support
- Modify index.html to register service worker
- Add GitHub Actions workflow for automated builds and deployment

## Implementation

- Workflow triggers on push to main
- Builds Rust/WASM with Trunk
- Deploys to gh-pages branch
- Caches dependencies (2-3 min build time)

## Testing

- Local build verified
- Service worker files present in dist/
- Ready for first deployment

## Next Steps

After merge, configure GitHub Pages:
1. Go to repo Settings → Pages
2. Source: Deploy from branch
3. Branch: gh-pages / (root)
4. Save

Site will be live at: https://gertalot.github.io/fractalwonder

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

Expected: PR created

**Step 4: Merge pull request**

After review and approval:

```bash
gh pr merge --squash --delete-branch
```

Expected: PR merged, feature branch deleted

**Step 5: Switch back to main**

Run:
```bash
git checkout main
git pull origin main
```

Expected: On main branch with latest changes

**Step 6: Wait for workflow to complete**

Run:
```bash
gh run list --limit 1
```

Monitor status until workflow completes (~5-8 minutes first run)

Expected: Workflow status "completed" with green checkmark

**Step 7: Configure GitHub Pages (one-time)**

Navigate to: `https://github.com/gertalot/fractalwonder/settings/pages`

Configure:
1. **Source:** Deploy from a branch
2. **Branch:** gh-pages
3. **Directory:** / (root)
4. Click **Save**

Expected: GitHub Pages configuration saved

**Step 8: Verify deployment**

Wait 1-2 minutes, then visit:
```
https://gertalot.github.io/fractalwonder
```

Expected: Site loads, check browser console for:
- Service worker registration messages
- SharedArrayBuffer availability
- App functions correctly

---

## Verification Checklist

After all tasks complete:

- [ ] coi-serviceworker files in static/ directory
- [ ] index.html includes service worker script
- [ ] GitHub Actions workflow in .github/workflows/deploy.yml
- [ ] All commits pushed to main
- [ ] Workflow completes successfully
- [ ] gh-pages branch created automatically
- [ ] GitHub Pages configured to serve from gh-pages
- [ ] Site live at https://gertalot.github.io/fractalwonder
- [ ] Service worker registers (check browser console)
- [ ] SharedArrayBuffer available (check: `typeof SharedArrayBuffer !== 'undefined'`)
- [ ] App renders fractals correctly

## Troubleshooting

**Build fails in CI:**
- Check Actions tab for logs
- Verify Cargo.lock is committed
- Check rust-toolchain version compatibility

**Service worker doesn't register:**
- Check browser console for errors
- Verify files copied to dist/ correctly
- Hard reload (Cmd+Shift+R) to clear cache

**SharedArrayBuffer undefined:**
- Service worker needs one page refresh on first visit
- Check private/incognito mode (may block service workers)
- Verify both .js files present in deployed site

**Site not updating:**
- Clear browser cache
- Check workflow completed successfully
- Verify gh-pages branch updated (check last commit time)

---

## Related Skills

- @superpowers:verification-before-completion - Use before claiming tasks complete
- @superpowers:systematic-debugging - Use if errors occur during implementation

## Success Criteria

1. Push to main triggers automated build
2. Build completes in 2-3 minutes (with caching)
3. Site deploys automatically to GitHub Pages
4. SharedArrayBuffer works (Web Workers function)
5. No manual intervention required for deployments
