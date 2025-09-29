import { Page, expect, Browser } from '@playwright/test';

/**
 * Get the main page from the connected browser context
 */
export async function getMainPage(): Promise<Page> {
  const browser: Browser = (globalThis as any).__HIELO_BROWSER__;
  if (!browser) {
    throw new Error('Browser not connected. Make sure global setup ran successfully.');
  }

  if (!browser.isConnected()) {
    throw new Error('Browser not available - using process-only testing mode');
  }

  const contexts = browser.contexts();
  if (contexts.length === 0) {
    // Create a new context for testing
    const context = await browser.newContext();
    return await context.newPage();
  }

  const pages = contexts[0].pages();
  if (pages.length === 0) {
    // Create a new page for testing if none exists
    const context = contexts[0];
    return await context.newPage();
  }

  return pages[0];
}

/**
 * Setup a test catalog connection (mocked for now)
 */
export async function setupTestCatalog(page: Page) {
  console.log('Setting up test catalog...');

  // Wait for the application to be ready
  await page.waitForLoadState('networkidle');

  // Check if we're already connected (catalog list visible)
  const catalogListVisible = await page.locator('[data-testid="catalog-list"], .catalog-tree').isVisible().catch(() => false);

  if (!catalogListVisible) {
    console.log('Not connected yet, attempting catalog connection...');

    // Look for catalog connection form elements
    // These selectors are based on common patterns, may need adjustment
    const nameInput = page.locator('input').filter({ hasText: /name/i }).or(
      page.locator('input[placeholder*="name" i]')
    ).first();

    const uriInput = page.locator('input').filter({ hasText: /uri|url/i }).or(
      page.locator('input[placeholder*="uri" i], input[placeholder*="url" i]')
    ).first();

    // Fill in test catalog details
    if (await nameInput.isVisible({ timeout: 5000 }).catch(() => false)) {
      await nameInput.fill('test-catalog');
      console.log('Filled catalog name');
    }

    if (await uriInput.isVisible({ timeout: 5000 }).catch(() => false)) {
      await uriInput.fill('http://localhost:8181');
      console.log('Filled catalog URI');
    }

    // Look for connect/add button
    const connectButton = page.locator('button').filter({ hasText: /connect|add/i }).first();
    if (await connectButton.isVisible({ timeout: 5000 }).catch(() => false)) {
      await connectButton.click();
      console.log('Clicked connect button');

      // Wait for connection to complete (or fail)
      try {
        await expect(page.locator('[data-testid="catalog-list"], .catalog-tree')).toBeVisible({ timeout: 10000 });
        console.log('✅ Catalog connection successful');
      } catch (e) {
        console.log('⚠️ Catalog connection may have failed, continuing anyway...');
      }
    }
  } else {
    console.log('Already connected to catalog');
  }
}

/**
 * Navigate to a specific table in the catalog tree
 */
export async function navigateToTable(page: Page, catalog: string, namespace: string, table: string) {
  console.log(`Navigating to table: ${catalog}.${namespace}.${table}`);

  // Expand catalog
  const catalogNode = page.locator('[data-testid="catalog-node"]').filter({ hasText: catalog }).or(
    page.locator('.catalog-tree-item').filter({ hasText: catalog })
  ).first();

  if (await catalogNode.isVisible({ timeout: 5000 }).catch(() => false)) {
    await catalogNode.click();
    console.log(`Expanded catalog: ${catalog}`);
  }

  // Wait for namespaces to load and expand namespace
  await page.waitForTimeout(2000);
  const namespaceNode = page.locator('[data-testid="namespace-node"]').filter({ hasText: namespace }).or(
    page.locator('.namespace-tree-item').filter({ hasText: namespace })
  ).first();

  if (await namespaceNode.isVisible({ timeout: 5000 }).catch(() => false)) {
    await namespaceNode.click();
    console.log(`Expanded namespace: ${namespace}`);
  }

  // Wait for tables to load and select table
  await page.waitForTimeout(2000);
  const tableNode = page.locator('[data-testid="table-item"]').filter({ hasText: table }).or(
    page.locator('.table-tree-item').filter({ hasText: table })
  ).first();

  if (await tableNode.isVisible({ timeout: 5000 }).catch(() => false)) {
    await tableNode.click();
    console.log(`Selected table: ${table}`);

    // Wait for table tabs to appear
    await expect(page.locator('[role="tab"]').filter({ hasText: /overview|schema/i })).toBeVisible({ timeout: 5000 });
  } else {
    console.log(`⚠️ Could not find table: ${table}`);
  }
}

/**
 * Wait for element with custom timeout and better error messages
 */
export async function waitForElement(page: Page, selector: string, timeout = 5000) {
  try {
    await page.locator(selector).waitFor({ timeout });
    return true;
  } catch (error) {
    console.log(`⚠️ Element not found: ${selector} (timeout: ${timeout}ms)`);
    return false;
  }
}

/**
 * Take a screenshot for debugging
 */
export async function debugScreenshot(page: Page, name: string) {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const filename = `debug-${name}-${timestamp}.png`;
  await page.screenshot({ path: `test-results/${filename}`, fullPage: true });
  console.log(`📸 Debug screenshot saved: ${filename}`);
}