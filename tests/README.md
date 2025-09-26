# Hielo UI Tests

This directory contains end-to-end UI tests for the Hielo desktop application using Playwright.

## Setup

1. **Install Node.js dependencies:**
   ```bash
   cd tests
   npm install
   npm run install-browsers
   ```

2. **Build Hielo:**
   ```bash
   # From the root directory
   cargo build
   ```

## Running Tests

### Local Development
```bash
# Run all tests
cd tests && npm test

# Run tests in headed mode (visible browser)
cd tests && npm run test:headed

# Debug a specific test
cd tests && npm run test:debug -- --grep "startup"

# Generate and view test report
cd tests && npm run test:report
```

### Platform-Specific Instructions

**Linux:**
```bash
# Install system dependencies
sudo apt-get install -y xvfb libnss3-dev libxss1 libasound2

# Run with virtual display
export DISPLAY=:99.0
xvfb-run -a npm test
```

**Windows:**
```bash
# No special setup required
npm test
```

**macOS:**
```bash
# Limited support - may require manual testing
npm test
```

## Test Structure

- `specs/`: Test files organized by feature area
- `utils/`: Shared test utilities and helpers
- `fixtures/`: Test data and mock responses
- `screenshots/`: Visual regression test baselines

## Current Test Coverage

### Phase 1 (Foundation) - ✅ Implemented
- **Application Startup**: Basic app launch and UI element detection
- **Basic Functionality**: Catalog interface detection and interaction testing
- **WebView Integration**: Playwright connection via Chrome DevTools Protocol

### Planned Phases
- **Phase 2**: Core workflow coverage (catalog connection, navigation, table viewing)
- **Phase 3**: CI/CD integration with GitHub Actions
- **Phase 4**: Visual regression testing with screenshots
- **Phase 5**: AccessKit-based testing for Rust-native approach

## Architecture

### WebView Debugging Setup
The tests connect to Hielo via Chrome DevTools Protocol (CDP):

1. **App Launch**: Hielo starts with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"`
2. **CDP Connection**: Playwright connects to `http://localhost:9222`
3. **Test Execution**: Tests run against the connected WebView instance
4. **Cleanup**: App process and browser connections are properly terminated

### Test Utilities

- `getMainPage()`: Get the main application page from the connected browser
- `setupTestCatalog()`: Attempt to connect to a test catalog
- `navigateToTable()`: Navigate to a specific table in the catalog tree
- `debugScreenshot()`: Take screenshots for debugging failed tests

## Debugging Failed Tests

1. **View test report:** `npm run test:report`
2. **Check screenshots:** Look in `test-results/` for failure screenshots
3. **Run in headed mode:** `npm run test:headed` to see the UI
4. **Debug specific test:** `npm run test:debug -- --grep "test name"`
5. **Check console output:** Tests include detailed logging of actions taken

## Writing New Tests

See existing tests in `specs/` for patterns. Key guidelines:

- Use flexible selectors that work with the actual Dioxus component structure
- Include debug logging and screenshots for troubleshooting
- Handle timing issues with appropriate waits
- Use the test utilities for common operations
- Test both success and error scenarios

### Example Test Structure
```typescript
import { test, expect } from '@playwright/test';
import { getMainPage, debugScreenshot } from '../utils/test-helpers.js';

test.describe('Feature Name', () => {
  test('should do something specific', async () => {
    const page = await getMainPage();

    // Take debug screenshot
    await debugScreenshot(page, 'feature-start');

    // Perform test actions
    await page.locator('button').first().click();

    // Verify results
    await expect(page.locator('.result')).toBeVisible();
  });
});
```

## Troubleshooting

### Common Issues

1. **Connection Failed**: Ensure Hielo builds successfully with `cargo build`
2. **Tests Timeout**: Increase timeout in `playwright.config.ts` or check app startup logs
3. **Element Not Found**: Use `debugScreenshot()` to see actual page structure
4. **Port Conflicts**: Change the debugging port in global-setup.ts if 9222 is in use

### Debug Commands
```bash
# Check if Hielo builds
cargo build

# Run a single test with debug output
npm run test:debug -- --grep "startup"

# View detailed test results
npm run test:report
```

## Next Steps

To extend test coverage:

1. **Add more specific selectors** based on actual component data-testid attributes
2. **Create mock catalog responses** for reliable test data
3. **Add visual regression tests** with screenshot comparisons
4. **Integrate with CI/CD** for automated testing
5. **Add AccessKit tests** for better cross-platform reliability