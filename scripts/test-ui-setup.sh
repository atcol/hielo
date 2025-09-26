#!/usr/bin/env bash
set -e

echo "🧪 Testing Hielo UI Test Foundation Setup"
echo "=========================================="

# Test 1: Verify Hielo builds successfully
echo "1. Testing Hielo build..."
if cargo build; then
    echo "✅ Hielo builds successfully"
else
    echo "❌ Hielo build failed"
    exit 1
fi

# Test 2: Check test structure
echo ""
echo "2. Verifying test directory structure..."
if [ -d "tests" ]; then
    echo "✅ tests/ directory exists"
else
    echo "❌ tests/ directory missing"
    exit 1
fi

required_dirs=("tests/specs" "tests/utils" "tests/fixtures" "tests/screenshots")
for dir in "${required_dirs[@]}"; do
    if [ -d "$dir" ]; then
        echo "✅ $dir exists"
    else
        echo "❌ $dir missing"
        exit 1
    fi
done

# Test 3: Check test configuration files
echo ""
echo "3. Checking test configuration files..."
required_files=(
    "tests/package.json"
    "tests/playwright.config.ts"
    "tests/global-setup.ts"
    "tests/global-teardown.ts"
    "tests/README.md"
)

for file in "${required_files[@]}"; do
    if [ -f "$file" ]; then
        echo "✅ $file exists"
    else
        echo "❌ $file missing"
        exit 1
    fi
done

# Test 4: Check test files
echo ""
echo "4. Checking test files..."
test_files=(
    "tests/specs/app-startup.spec.ts"
    "tests/specs/basic-functionality.spec.ts"
    "tests/utils/test-helpers.ts"
)

for file in "${test_files[@]}"; do
    if [ -f "$file" ]; then
        echo "✅ $file exists"
    else
        echo "❌ $file missing"
        exit 1
    fi
done

# Test 5: Verify WebView debugging environment
echo ""
echo "5. Testing WebView debugging setup..."
echo "   Setting WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS..."
export WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"
export HIELO_CONFIG_DIR="/tmp/hielo-test-config"

echo "✅ Environment variables configured for testing"

# Test 6: Brief functionality test
echo ""
echo "6. Testing Hielo startup (5 second test)..."
echo "   Starting Hielo with debugging enabled..."

# Start Hielo in background for a brief test
./target/debug/hielo &
HIELO_PID=$!

echo "   Hielo started with PID: $HIELO_PID"
echo "   Waiting 5 seconds to test stability..."
sleep 5

# Check if process is still running
if kill -0 $HIELO_PID 2>/dev/null; then
    echo "✅ Hielo running successfully with debug configuration"
    kill $HIELO_PID
    wait $HIELO_PID 2>/dev/null || true
else
    echo "❌ Hielo process died during startup"
    exit 1
fi

echo ""
echo "🎉 All tests passed! UI test foundation is ready."
echo ""
echo "Next steps:"
echo "1. Install Node.js and npm in your development environment"
echo "2. Run: cd tests && npm install && npm run install-browsers"
echo "3. Run: cd tests && npm test"
echo ""
echo "For CI/CD integration, see the GitHub Actions workflow in the implementation plan."