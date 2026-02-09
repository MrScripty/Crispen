#!/bin/bash
# Crispen Launcher
# Builds and runs the Bevy + Svelte color grading application

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

UI_DIR="crates/crispen-demo/ui"

# Parse arguments
BUILD_ONLY=false
RUN_ONLY=false
RELEASE=false
DEV_MODE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --build)
            BUILD_ONLY=true
            shift
            ;;
        --run)
            RUN_ONLY=true
            shift
            ;;
        --release)
            RELEASE=true
            shift
            ;;
        --dev)
            DEV_MODE=true
            shift
            ;;
        --help|-h)
            echo "Crispen Launcher"
            echo ""
            echo "Usage: ./launcher.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --build     Build only (UI + Rust), don't run"
            echo "  --run       Run only (skip build, use existing binary)"
            echo "  --release   Build and run in release mode"
            echo "  --dev       Development mode (Vite dev server for UI hot-reload)"
            echo "  --help, -h  Show this help message"
            echo ""
            echo "With no options, builds everything and runs in debug mode."
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Resolve binary path
if [ "$RELEASE" = true ]; then
    BINARY="target/release/crispen-demo"
else
    BINARY="target/debug/crispen-demo"
fi

# --- Build ---
if [ "$RUN_ONLY" = false ]; then
    # Build Svelte UI (skip in dev mode — Vite serves it live)
    if [ "$DEV_MODE" = false ]; then
        echo "Building Svelte UI..."
        cd "$UI_DIR"
        if [ ! -d "node_modules" ]; then
            echo "Installing npm dependencies..."
            npm install
        fi
        npm run build
        cd "$SCRIPT_DIR"
    fi

    # Build Rust workspace
    echo "Building Crispen..."
    if [ "$RELEASE" = true ]; then
        cargo build --release -p crispen-demo
    else
        cargo build -p crispen-demo
    fi
fi

if [ "$BUILD_ONLY" = true ]; then
    echo "Build complete: $BINARY"
    exit 0
fi

# --- Run ---
if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found at $BINARY"
    echo "Run without --run to build first, or check for build errors."
    exit 1
fi

echo "Launching Crispen..."

if [ "$DEV_MODE" = true ]; then
    export CRISPEN_DEV=1
    echo "(Development mode — UI served from Vite dev server)"
    echo "Start Vite in another terminal: cd $UI_DIR && npm run dev"
fi

exec "$BINARY"
