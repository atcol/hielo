import { spawn, ChildProcess } from 'child_process';
import { chromium, Browser } from '@playwright/test';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let appProcess: ChildProcess;
let browser: Browser;

async function globalSetup() {
  console.log('🔧 Setting up Hielo UI test environment...');

  // Build Hielo in debug mode for testing
  console.log('Building Hielo for testing...');
  await new Promise<void>((resolve, reject) => {
    const buildProcess = spawn('cargo', ['build'], {
      cwd: path.join(__dirname, '..'),
      stdio: 'inherit'
    });
    buildProcess.on('close', (code) => {
      if (code === 0) {
        console.log('✅ Build completed successfully');
        resolve();
      } else {
        reject(new Error(`Build failed with code ${code}`));
      }
    });
  });

  // Determine executable path based on platform
  const platform = process.platform;
  const executablePath = platform === 'win32'
    ? path.join(__dirname, '..', 'target', 'debug', 'hielo.exe')
    : path.join(__dirname, '..', 'target', 'debug', 'hielo');

  console.log(`Starting Hielo from: ${executablePath}`);

  // Start Hielo with WebView debugging enabled
  console.log('🚀 Starting Hielo with debugging enabled...');
  appProcess = spawn(executablePath, [], {
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222 --no-sandbox --disable-web-security',
      WEBVIEW2_USER_DATA_FOLDER: '/tmp/hielo-testing',
      // Prevent normal config loading during tests
      HIELO_CONFIG_DIR: '/tmp/hielo-test-config',
      // Disable logging noise
      RUST_LOG: 'error',
      // Virtual display for headless environments
      DISPLAY: process.env.DISPLAY || ':99'
    },
    stdio: ['pipe', 'pipe', 'pipe']
  });

  // Handle app process output for debugging
  appProcess.stdout?.on('data', (data) => {
    console.log(`[Hielo stdout]: ${data}`);
  });

  appProcess.stderr?.on('data', (data) => {
    console.log(`[Hielo stderr]: ${data}`);
  });

  appProcess.on('error', (error) => {
    console.error('Failed to start Hielo process:', error);
  });

  // Wait for application to start and become ready
  console.log('⏳ Waiting for Hielo to start...');
  await new Promise(resolve => setTimeout(resolve, 8000)); // Give it more time

  // Attempt to connect Playwright to the running app
  console.log('🔌 Connecting Playwright to Hielo via CDP...');
  let retries = 5;
  while (retries > 0) {
    try {
      browser = await chromium.connectOverCDP('http://localhost:9222');
      console.log('✅ Connected to Hielo via CDP');

      // Verify we have access to the app
      const contexts = browser.contexts();
      console.log(`Found ${contexts.length} browser contexts`);

      if (contexts.length > 0) {
        const pages = contexts[0].pages();
        console.log(`Found ${pages.length} pages in context`);
      }

      break;
    } catch (error) {
      console.log(`Connection attempt failed (${retries} retries left):`, error.message);
      retries--;
      if (retries > 0) {
        await new Promise(resolve => setTimeout(resolve, 2000));
      } else {
        console.error('Failed to connect to Hielo after multiple attempts');
        appProcess?.kill();
        throw new Error('Could not connect to Hielo application via CDP');
      }
    }
  }

  // Store references for cleanup
  (globalThis as any).__HIELO_PROCESS__ = appProcess;
  (globalThis as any).__HIELO_BROWSER__ = browser;

  return async () => {
    console.log('🧹 Cleaning up test environment...');
    try {
      await browser?.close();
    } catch (e) {
      console.log('Error closing browser:', e.message);
    }

    try {
      appProcess?.kill();
    } catch (e) {
      console.log('Error killing app process:', e.message);
    }
  };
}

export default globalSetup;