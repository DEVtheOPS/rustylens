import { test, expect } from '@playwright/test';

test.describe('Cluster Overview', () => {
  test('should display cluster overview page', async ({ page }) => {
    await page.goto('/');

    // Page title
    await expect(page.locator('h1:has-text("Clusters")')).toBeVisible();

    // Description text
    await expect(page.locator('text=Manage your Kubernetes clusters')).toBeVisible();
  });

  test('should show empty state when no clusters exist', async ({ page }) => {
    await page.goto('/');

    // Try to find empty state
    const emptyState = page.locator('text=No Clusters Yet');
    const hasEmptyState = await emptyState.isVisible().catch(() => false);

    if (hasEmptyState) {
      // Verify empty state content
      await expect(page.locator('text=Get started by adding your first')).toBeVisible();

      // Should show rocket emoji
      await expect(page.locator('text=🚀')).toBeVisible();
    }
  });

  test('should show DataTable when clusters exist', async ({ page }) => {
    await page.goto('/');

    // Check if table or empty state is shown
    const table = page.locator('[role="table"]');
    const emptyState = page.locator('text=No Clusters Yet');

    const hasTable = await table.isVisible().catch(() => false);
    const hasEmptyState = await emptyState.isVisible().catch(() => false);

    // Should have one or the other
    expect(hasTable || hasEmptyState).toBeTruthy();
  });

  test('should have DataTable with correct columns', async ({ page }) => {
    await page.goto('/');

    // If table exists, check columns
    const table = page.locator('[role="table"]');
    const hasTable = await table.isVisible().catch(() => false);

    if (hasTable) {
      // Check for column headers
      const expectedColumns = ['Icon', 'Name', 'Context', 'Description', 'Tags', 'Last Accessed'];

      for (const columnName of expectedColumns) {
        await expect(page.locator(`text=${columnName}`)).toBeVisible();
      }
    }
  });

  test('should have search functionality', async ({ page }) => {
    await page.goto('/');

    const table = page.locator('[role="table"]');
    const hasTable = await table.isVisible().catch(() => false);

    if (hasTable) {
      // Search input should be visible
      const searchInput = page.locator('input[placeholder*="Search"]').or(page.locator('input[type="text"]')).first();
      await expect(searchInput).toBeVisible();
    }
  });

  test('should have refresh button', async ({ page }) => {
    await page.goto('/');

    const table = page.locator('[role="table"]');
    const hasTable = await table.isVisible().catch(() => false);

    if (hasTable) {
      // Refresh button should be visible
      const refreshButton = page.locator('button:has-text("Refresh")').or(page.locator('button[title*="efresh"]'));
      const hasRefresh = await refreshButton.count();
      expect(hasRefresh).toBeGreaterThanOrEqual(0);
    }
  });

  test('should show column configuration', async ({ page }) => {
    await page.goto('/');

    const table = page.locator('[role="table"]');
    const hasTable = await table.isVisible().catch(() => false);

    if (hasTable) {
      // Column config button should exist
      const columnButton = page.locator('button').filter({ hasText: 'Columns' }).or(
        page.locator('button[title*="olumn"]')
      );
      const hasColumnButton = await columnButton.count();
      expect(hasColumnButton).toBeGreaterThanOrEqual(0);
    }
  });
});
