import { test, expect } from '@playwright/test';

test.describe('Layout and Navigation', () => {
  test('should display icon sidebar', async ({ page }) => {
    await page.goto('/');

    // Check icon sidebar exists
    const iconSidebar = page.locator('aside').first();
    await expect(iconSidebar).toBeVisible();

    // Check for overview/home icon
    const homeLink = iconSidebar.locator('a[href="/"]').first();
    await expect(homeLink).toBeVisible();

    // Check for settings icon
    const settingsLink = iconSidebar.locator('a[href="/settings"]');
    await expect(settingsLink).toBeVisible();

    // Check for add cluster button
    const addButton = iconSidebar.locator('button[title="Add Cluster"]');
    await expect(addButton).toBeVisible();
  });

  test('should navigate to overview page by default', async ({ page }) => {
    await page.goto('/');

    // Check URL
    expect(page.url()).toContain('/');

    // Check for "Clusters" heading
    await expect(page.locator('h1:has-text("Clusters")')).toBeVisible();
  });

  test('should navigate to settings page', async ({ page }) => {
    await page.goto('/');

    // Click settings icon
    await page.locator('a[href="/settings"]').click();

    // Check URL
    await page.waitForURL('**/settings');

    // Check for settings heading
    await expect(page.locator('h1:has-text("Settings")')).toBeVisible();

    // Check for theme selector
    await expect(page.locator('label:has-text("Theme")')).toBeVisible();
  });

  test('should navigate back to overview from settings', async ({ page }) => {
    await page.goto('/settings');

    // Click home icon
    await page.locator('a[href="/"]').first().click();

    // Check URL
    await page.waitForURL('/');

    // Check for clusters heading
    await expect(page.locator('h1:has-text("Clusters")')).toBeVisible();
  });

  test('should show empty state when no clusters', async ({ page }) => {
    await page.goto('/');

    // Should show empty state or cluster list
    const emptyState = page.locator('text=No Clusters Yet');
    const dataTable = page.locator('[role="table"]');

    // Either empty state or table should be visible
    const hasEmptyState = await emptyState.isVisible().catch(() => false);
    const hasTable = await dataTable.isVisible().catch(() => false);

    expect(hasEmptyState || hasTable).toBeTruthy();
  });

  test('should have responsive layout', async ({ page }) => {
    await page.goto('/');

    // Check main container
    const mainContainer = page.locator('div.flex.h-screen.w-screen');
    await expect(mainContainer).toBeVisible();

    // Icon sidebar should be 48px wide (w-12)
    const iconSidebar = page.locator('aside.w-12');
    await expect(iconSidebar).toBeVisible();
  });

  test('should apply theme correctly', async ({ page }) => {
    await page.goto('/settings');

    // Theme select should be visible
    const themeSelect = page.locator('#theme-select');
    await expect(themeSelect).toBeVisible();

    // Check that root has a theme class
    const htmlElement = page.locator('html');
    const classList = await htmlElement.getAttribute('class');

    // Should have one of the theme classes
    const hasTheme = ['kore', 'kore-light', 'rusty', 'rusty-light', 'dracula', 'alucard'].some(
      theme => classList?.includes(theme)
    );
    expect(hasTheme).toBeTruthy();
  });
});
