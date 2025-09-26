import { test, expect } from '@playwright/test';
import { getMainPage, debugScreenshot, waitForElement } from '../utils/test-helpers.js';

test.describe('Application Startup', () => {
  test('should launch and show the application interface', async () => {
    const page = await getMainPage();

    // Take a debug screenshot to see what we're working with
    await debugScreenshot(page, 'startup');

    // The app should have loaded and show some content
    // We'll be flexible about what exactly we see since this is the first test
    await expect(page.locator('body')).not.toBeEmpty();

    // Check for basic HTML structure
    const hasContent = await page.locator('div, button, input, h1, h2, h3, span').count();
    expect(hasContent).toBeGreaterThan(0);

    console.log(`✅ Found ${hasContent} UI elements on the page`);
  });

  test('should have basic UI elements', async () => {
    const page = await getMainPage();

    // Look for common UI patterns in the app
    // These are flexible selectors to catch various possible structures

    // Should have some form of navigation or main content
    const hasNavigation = await page.locator('[role="navigation"], nav, .navigation, .nav').count() > 0;
    const hasMainContent = await page.locator('[role="main"], main, .main-content').count() > 0;
    const hasButtons = await page.locator('button').count() > 0;
    const hasInputs = await page.locator('input').count() > 0;

    console.log('UI Elements found:');
    console.log(`- Navigation: ${hasNavigation}`);
    console.log(`- Main content: ${hasMainContent}`);
    console.log(`- Buttons: ${hasButtons}`);
    console.log(`- Inputs: ${hasInputs}`);

    // At least some interactive elements should be present
    expect(hasButtons || hasInputs).toBe(true);
  });

  test('should be responsive to basic interactions', async () => {
    const page = await getMainPage();

    // Try to interact with the first visible button or input
    const firstButton = page.locator('button').first();
    const firstInput = page.locator('input').first();

    if (await firstButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      // Test that we can focus/hover over the button without errors
      await firstButton.hover();
      console.log('✅ Successfully hovered over first button');
    }

    if (await firstInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      // Test that we can focus the input without errors
      await firstInput.focus();
      console.log('✅ Successfully focused first input');
    }

    // The page should remain responsive
    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle window title and basic metadata', async () => {
    const page = await getMainPage();

    // Check if we can access basic page properties
    const title = await page.title();
    console.log(`Page title: "${title}"`);

    // Title should not be empty (Dioxus apps typically have some title)
    expect(title.length).toBeGreaterThan(0);

    // Check viewport is reasonable
    const viewport = page.viewportSize();
    if (viewport) {
      expect(viewport.width).toBeGreaterThan(300);
      expect(viewport.height).toBeGreaterThan(200);
      console.log(`Viewport: ${viewport.width}x${viewport.height}`);
    }
  });
});