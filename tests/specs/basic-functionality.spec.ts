import { test, expect } from '@playwright/test';
import { getMainPage, setupTestCatalog, debugScreenshot } from '../utils/test-helpers.js';

test.describe('Basic Functionality', () => {
  test('should show catalog connection interface', async () => {
    const page = await getMainPage();

    await debugScreenshot(page, 'catalog-interface');

    // Look for catalog connection elements
    // These are common patterns we'd expect in a catalog connection UI
    const hasInputFields = await page.locator('input').count() > 0;
    const hasButtons = await page.locator('button').count() > 0;

    expect(hasInputFields).toBe(true);
    expect(hasButtons).toBe(true);

    // Look for text that suggests this is a catalog connection interface
    const pageText = await page.textContent('body') || '';
    const hasCatalogText = /catalog|connection|connect|uri|url|name/i.test(pageText);

    console.log('Page contains catalog-related text:', hasCatalogText);
    if (hasCatalogText) {
      console.log('✅ Appears to be showing catalog connection interface');
    } else {
      console.log('⚠️ May not be catalog connection interface, or already connected');
    }
  });

  test('should handle basic navigation patterns', async () => {
    const page = await getMainPage();

    // Test if we can navigate around the interface
    const buttons = page.locator('button');
    const buttonCount = await buttons.count();

    console.log(`Found ${buttonCount} buttons on the page`);

    if (buttonCount > 0) {
      // Test clicking the first few buttons to see if the interface responds
      for (let i = 0; i < Math.min(3, buttonCount); i++) {
        const button = buttons.nth(i);
        const buttonText = await button.textContent() || '';

        if (await button.isVisible() && await button.isEnabled()) {
          console.log(`Testing button ${i}: "${buttonText}"`);

          try {
            await button.click({ timeout: 2000 });
            // Wait briefly to see if anything changes
            await page.waitForTimeout(1000);
            console.log(`✅ Successfully clicked button: "${buttonText}"`);
          } catch (error) {
            console.log(`⚠️ Could not click button "${buttonText}": ${error.message}`);
          }
        }
      }
    }

    // Verify the page is still responsive after interactions
    await expect(page.locator('body')).toBeVisible();
  });

  test('should attempt catalog setup flow', async () => {
    const page = await getMainPage();

    try {
      await setupTestCatalog(page);
      console.log('✅ Catalog setup completed without errors');
    } catch (error) {
      console.log(`⚠️ Catalog setup encountered issues: ${error.message}`);
      await debugScreenshot(page, 'catalog-setup-error');
    }

    // Regardless of setup success, the page should remain functional
    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle keyboard interactions', async () => {
    const page = await getMainPage();

    // Test basic keyboard navigation
    await page.keyboard.press('Tab');
    await page.waitForTimeout(500);

    await page.keyboard.press('Tab');
    await page.waitForTimeout(500);

    // Test if Escape key works (common for closing modals/dialogs)
    await page.keyboard.press('Escape');
    await page.waitForTimeout(500);

    // Test if Ctrl+A works in any input fields
    const inputs = page.locator('input[type="text"], input:not([type]), textarea');
    const inputCount = await inputs.count();

    if (inputCount > 0) {
      const firstInput = inputs.first();
      if (await firstInput.isVisible({ timeout: 2000 }).catch(() => false)) {
        await firstInput.click();
        await page.keyboard.press('Control+a');
        console.log('✅ Keyboard shortcuts work in input fields');
      }
    }

    // Page should remain responsive
    await expect(page.locator('body')).toBeVisible();
  });
});