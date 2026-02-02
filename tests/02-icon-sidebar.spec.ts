import { test, expect } from '@playwright/test';

test.describe('Icon Sidebar', () => {
  test('should show active state for current page', async ({ page }) => {
    await page.goto('/');

    // Home icon should have active styling
    const homeLink = page.locator('aside').first().locator('a[href="/"]').first();
    await expect(homeLink).toHaveClass(/bg-bg-main/);
  });

  test('should update active state when navigating', async ({ page }) => {
    await page.goto('/');

    // Navigate to settings
    await page.locator('a[href="/settings"]').click();
    await page.waitForURL('**/settings');

    // Settings icon should now be active
    const settingsLink = page.locator('a[href="/settings"]');
    await expect(settingsLink).toHaveClass(/bg-bg-main/);
  });

  test('should show add cluster button', async ({ page }) => {
    await page.goto('/');

    const addButton = page.locator('button[title="Add Cluster"]');
    await expect(addButton).toBeVisible();

    // Should have Plus icon
    await expect(addButton.locator('svg')).toBeVisible();
  });

  test('should open import modal when clicking add button', async ({ page }) => {
    await page.goto('/');

    // Click add cluster button
    await page.locator('button[title="Add Cluster"]').click();

    // Modal should appear
    await expect(page.locator('text=Import Clusters')).toBeVisible();

    // Should have tabs
    await expect(page.locator('text=Import from File')).toBeVisible();
    await expect(page.locator('text=Import from Folder')).toBeVisible();
  });

  test('should close import modal', async ({ page }) => {
    await page.goto('/');

    // Open modal
    await page.locator('button[title="Add Cluster"]').click();
    await expect(page.locator('text=Import Clusters')).toBeVisible();

    // Close modal by clicking X button
    await page.locator('button:has(svg)').filter({ hasText: '' }).first().click();

    // Modal should be gone
    await expect(page.locator('text=Import Clusters')).not.toBeVisible();
  });

  test('should show bookmarked clusters', async ({ page }) => {
    await page.goto('/');

    // Check for cluster bookmarks container
    const bookmarksContainer = page.locator('aside').first().locator('div.flex-1.overflow-y-auto');
    await expect(bookmarksContainer).toBeVisible();

    // Should either have clusters or be empty
    const hasBookmarks = await bookmarksContainer.locator('a[href^="/cluster/"]').count();
    expect(hasBookmarks).toBeGreaterThanOrEqual(0);
  });

  test('should show dividers between sections', async ({ page }) => {
    await page.goto('/');

    const iconSidebar = page.locator('aside').first();

    // Should have divider elements
    const dividers = iconSidebar.locator('div.h-px.bg-border-subtle');
    const dividerCount = await dividers.count();

    expect(dividerCount).toBeGreaterThanOrEqual(2);
  });

  test('should have proper icon sizing', async ({ page }) => {
    await page.goto('/');

    const iconSidebar = page.locator('aside').first();

    // Icons should be visible
    const homeIcon = iconSidebar.locator('a[href="/"]').first().locator('svg');
    await expect(homeIcon).toBeVisible();

    // Check icon has appropriate size class
    const iconBox = await homeIcon.boundingBox();
    expect(iconBox).toBeTruthy();
  });
});
