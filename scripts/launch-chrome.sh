#!/bin/bash
# Launch Chrome with remote debugging enabled for Claude Code devcontainer
# This script should be run on the HOST (not inside the container)

set -e

# Default port
PORT=9223

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --port)
      PORT="$2"
      shift 2
      ;;
    *)
      echo "Unknown option: $1"
      echo "Usage: $0 [--port PORT]"
      exit 1
      ;;
  esac
done

# Detect Chrome binary location based on OS
detect_chrome() {
    # macOS
    if [ -f "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" ]; then
        echo "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
        return 0
    fi

    # Linux - try common locations
    for chrome_path in \
        "/usr/bin/google-chrome" \
        "/usr/bin/google-chrome-stable" \
        "/usr/bin/chromium" \
        "/usr/bin/chromium-browser" \
        "/snap/bin/chromium"; do
        if [ -f "$chrome_path" ]; then
            echo "$chrome_path"
            return 0
        fi
    done

    # Windows (Git Bash/WSL)
    if command -v chrome.exe &> /dev/null; then
        echo "chrome.exe"
        return 0
    fi

    return 1
}

# Find Chrome
CHROME_PATH=$(detect_chrome)

if [ -z "$CHROME_PATH" ]; then
    echo "Error: Could not find Chrome/Chromium installation."
    echo ""
    echo "Please install Google Chrome or Chromium:"
    echo "  macOS: brew install --cask google-chrome"
    echo "  Ubuntu/Debian: sudo apt install google-chrome-stable"
    echo "  Or download from: https://www.google.com/chrome/"
    exit 1
fi

echo "Found Chrome at: $CHROME_PATH"
echo ""
echo "Starting Chrome with remote debugging on port $PORT..."
echo ""
echo "IMPORTANT: This Chrome instance is configured for debugging and should"
echo "NOT be used for regular browsing or handling sensitive data."
echo ""
echo "To use with Claude Code devcontainer, add this to .mcp.json:"
echo ""
echo '{'
echo '  "mcpServers": {'
echo '    "chrome-devtools": {'
echo '      "command": "npx",'
echo '      "args": ['
echo '        "-y",'
echo '        "chrome-devtools-mcp@latest",'
echo "        \"--browserUrl=http://localhost:$PORT\","
echo '        "--logFile=/tmp/chrome-devtools-mcp.log"'
echo '      ]'
echo '    }'
echo '  }'
echo '}'
echo ""

# Set up user data directory
# FIXED 2025-10-24: Copied Local Storage + WebStorage from chrome-playwright
# This contains the Developer Mode authorization for Word for Web
USER_DATA_DIR="${HOME}/.config/chrome-devtools-fractalwonder"
mkdir -p "$USER_DATA_DIR"

# Launch Chrome with debugging enabled
"$CHROME_PATH" \
  --remote-debugging-port="$PORT" \
  --allow-insecure-localhost \
  --disable-web-security \
  --disable-features=IsolateOrigins,site-per-process \
  --user-data-dir="$USER_DATA_DIR" \
  --no-first-run \
  --no-default-browser-check
