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
NO_CEF=false

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
        --no-cef)
            NO_CEF=true
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
            echo "  --no-cef    Disable CEF (use WebSocket bridge fallback)"
            echo "  --help, -h  Show this help message"
            echo ""
            echo "With no options, builds everything and runs in debug mode with CEF."
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Resolve build type
if [ "$RELEASE" = true ]; then
    BUILD_TYPE="release"
    BINARY="target/release/crispen-demo"
    HELPER_BINARY="target/release/crispen-cef-helper"
else
    BUILD_TYPE="debug"
    BINARY="target/debug/crispen-demo"
    HELPER_BINARY="target/debug/crispen-cef-helper"
fi

# Resolve cargo features
if [ "$NO_CEF" = true ]; then
    CARGO_FEATURES="--no-default-features --features ocio"
else
    CARGO_FEATURES=""  # default features include cef
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
        cargo build --release -p crispen-demo $CARGO_FEATURES
        if [ "$NO_CEF" = false ]; then
            cargo build --release -p crispen-cef-helper
        fi
    else
        cargo build -p crispen-demo $CARGO_FEATURES
        if [ "$NO_CEF" = false ]; then
            cargo build -p crispen-cef-helper
        fi
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

# Set up CEF library path
if [ "$NO_CEF" = false ]; then
    # Find libcef.so from the cef-dll-sys build output
    CEF_LIB_DIR=$(find "target/$BUILD_TYPE/build" -type d -name "cef_linux_x86_64" 2>/dev/null | head -1)
    if [ -n "$CEF_LIB_DIR" ] && [ -f "$CEF_LIB_DIR/libcef.so" ]; then
        export LD_LIBRARY_PATH="$CEF_LIB_DIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
        echo "CEF libraries: $CEF_LIB_DIR"
    else
        echo "Warning: libcef.so not found in build output."
        echo "The cef crate should download it automatically during build."
        echo "Try rebuilding: ./launcher.sh"
    fi

    # Ensure helper binary is discoverable (same dir as main binary or via env)
    if [ -f "$HELPER_BINARY" ]; then
        export CEF_HELPER_PATH="$SCRIPT_DIR/$HELPER_BINARY"
    fi
fi

exec "$BINARY"
