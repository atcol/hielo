#!/bin/bash

# Virtual X server for headless GUI testing
# Start Xvfb (X Virtual Framebuffer) for GUI applications in headless environments

set -e

# Check if Xvfb is available
if ! command -v Xvfb &> /dev/null; then
    echo "⚠️  Xvfb not found - GUI tests may fail in headless environments"
    echo "Install with: apt-get install xvfb (Ubuntu/Debian) or yum install xorg-x11-server-Xvfb (RHEL/CentOS)"
fi

# Default display
DISPLAY_NUM=${DISPLAY_NUM:-99}
DISPLAY_VALUE=":${DISPLAY_NUM}"

# Screen resolution for virtual display
SCREEN_RES=${SCREEN_RES:-1920x1080x24}

# Check if display is already running
if xdpyinfo -display "$DISPLAY_VALUE" >/dev/null 2>&1; then
    echo "✅ Display $DISPLAY_VALUE already running"
else
    echo "🖥️  Starting virtual display $DISPLAY_VALUE with resolution $SCREEN_RES"

    # Start Xvfb in background
    Xvfb "$DISPLAY_VALUE" -screen 0 "$SCREEN_RES" -ac -nolisten tcp -dpi 96 &
    XVFB_PID=$!

    # Wait for Xvfb to start
    echo "⏳ Waiting for virtual display to start..."
    for i in {1..10}; do
        if xdpyinfo -display "$DISPLAY_VALUE" >/dev/null 2>&1; then
            echo "✅ Virtual display started successfully"
            break
        fi
        if [ $i -eq 10 ]; then
            echo "❌ Failed to start virtual display"
            kill $XVFB_PID 2>/dev/null || true
            exit 1
        fi
        sleep 1
    done
fi

# Export display for child processes
export DISPLAY="$DISPLAY_VALUE"

# Run the command with virtual display
echo "🚀 Running command with DISPLAY=$DISPLAY"
exec "$@"