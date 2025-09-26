async function globalTeardown() {
  console.log('🧹 Running global teardown...');

  try {
    // Close browser connection
    const browser = (globalThis as any).__HIELO_BROWSER__;
    if (browser) {
      await browser.close();
      console.log('✅ Browser connection closed');
    }
  } catch (error: any) {
    console.log('⚠️  Error closing browser:', error.message);
  }

  try {
    // Kill app process
    const appProcess = (globalThis as any).__HIELO_PROCESS__;
    if (appProcess && !appProcess.killed) {
      appProcess.kill('SIGTERM');

      // Give it a moment to close gracefully
      await new Promise(resolve => setTimeout(resolve, 2000));

      // Force kill if still running
      if (!appProcess.killed) {
        appProcess.kill('SIGKILL');
      }
      console.log('✅ Hielo process terminated');
    }
  } catch (error: any) {
    console.log('⚠️  Error killing app process:', error.message);
  }

  console.log('✅ Global teardown completed');
}

export default globalTeardown;