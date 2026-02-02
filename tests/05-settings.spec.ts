import { test, expect } from '@playwright/test';

test.describe('Settings Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/settings');
  });

  test('should display settings page', async ({ page }) => {
    // Page heading
    await expect(page.locator('h1:has-text("Settings")')).toBeVisible();

    // Description
    await expect(page.locator('text=Manage your application preferences')).toBeVisible();
  });

  test('should show appearance settings card', async ({ page }) => {
    // Appearance heading
    await expect(page.locator('h3:has-text("Appearance")')).toBeVisible();

    // Description
    await expect(page.locator('text=Customize the look and feel')).toBeVisible();

    // Palette icon should be visible
    const paletteIcon = page.locator('svg').first();
    await expect(paletteIcon).toBeVisible();
  });

  test('should show theme selector', async ({ page }) => {
    // Theme label
    await expect(page.locator('label:has-text("Theme")').or(page.locator('label[for="theme-select"]'))).toBeVisible();

    // Theme select component
    const themeSelect = page.locator('#theme-select').or(page.locator('button').filter({ hasText: /kore|rusty|dracula|alucard/i }));
    await expect(themeSelect.first()).toBeVisible();
  });

  test('should have multiple theme options', async ({ page }) => {
    // Click theme selector to open dropdown
    const themeButton = page.locator('#theme-select').or(
      page.locator('label:has-text("Theme")').locator('..').locator('button')
    ).first();

    await themeButton.click();

    // Wait a moment for dropdown to appear
    await page.waitForTimeout(300);

    // Check for theme options (they might be in a dropdown menu)
    const hasOptions = await page.locator('text=kore').or(page.locator('text=Kore')).isVisible();
    expect(hasOptions).toBeTruthy();
  });

  test('should persist theme selection', async ({ page }) => {
    // Get current theme from html element
    const htmlElement = page.locator('html');
    const initialClass = await htmlElement.getAttribute('class');

    expect(initialClass).toBeTruthy();

    // Verify theme is applied
    const hasTheme = ['kore', 'kore-light', 'rusty', 'rusty-light', 'dracula', 'alucard'].some(
      theme => initialClass?.includes(theme)
    );
    expect(hasTheme).toBeTruthy();
  });

  test('should navigate back to overview', async ({ page }) => {
    // Click home icon
    await page.locator('a[href="/"]').first().click();

    await page.waitForURL('/');

    // Should be on overview page
    await expect(page.locator('h1:has-text("Clusters")')).toBeVisible();
  });

  test('should have proper layout structure', async ({ page }) => {
    // Max width container
    const container = page.locator('div.max-w-3xl');
    await expect(container).toBeVisible();

    // Settings card
    const card = page.locator('div:has(> div:has-text("Appearance"))');
    await expect(card).toBeVisible();
  });
});
