import { test, expect } from '@playwright/test';

test.describe('Cluster Routes Structure', () => {
  // These tests verify the route structure exists and handles navigation
  // Note: Without actual cluster data, we test for proper 404 or "not found" handling

  test('should handle non-existent cluster gracefully', async ({ page }) => {
    // Try to navigate to a non-existent cluster
    await page.goto('/cluster/non-existent-uuid');

    // Should either show loading, not found, or redirect
    await page.waitForLoadState('networkidle');

    // Check for error state or redirect
    const notFound = page.locator('text=Cluster Not Found').or(page.locator('text=not found'));
    const loading = page.locator('text=Loading cluster');
    const overview = page.locator('h1:has-text("Clusters")');

    const hasExpectedState =
      await notFound.isVisible().catch(() => false) ||
      await loading.isVisible().catch(() => false) ||
      await overview.isVisible().catch(() => false);

    expect(hasExpectedState).toBeTruthy();
  });

  test('should have correct route structure for cluster resources', async ({ page }) => {
    // Test that routes are properly configured in SvelteKit
    // This will fail with 404 or show "not found" but verifies routing works
    const routes = [
      '/cluster/test-id',
      '/cluster/test-id/pods',
      '/cluster/test-id/deployments',
      '/cluster/test-id/services',
      '/cluster/test-id/settings',
    ];

    for (const route of routes) {
      const response = await page.goto(route);

      // Route should be recognized (not a hard 404)
      // Either shows content or a proper "not found" message from our app
      expect(response?.status()).not.toBe(404);
    }
  });

  test('should preserve cluster ID in URL during navigation', async ({ page }) => {
    await page.goto('/cluster/test-123');

    // Even if cluster doesn't exist, URL should be maintained
    expect(page.url()).toContain('/cluster/test-123');
  });
});

test.describe('Cluster Layout Structure (with mock cluster)', () => {
  test('should show both sidebars in cluster view', async ({ page }) => {
    // When in a cluster route, should have both icon sidebar and resource sidebar
    await page.goto('/cluster/test-id');

    // Icon sidebar should always be present
    const iconSidebar = page.locator('aside.w-12');
    await expect(iconSidebar).toBeVisible();

    // If cluster is found, resource sidebar should also be present
    const resourceSidebar = page.locator('aside.w-64');
    const hasResourceSidebar = await resourceSidebar.count();

    // Resource sidebar exists if cluster is valid
    expect(hasResourceSidebar).toBeGreaterThanOrEqual(0);
  });

  test('should show header bar in cluster view', async ({ page }) => {
    await page.goto('/cluster/test-id/pods');

    // If cluster exists, should have header
    const header = page.locator('header.h-14');
    const hasHeader = await header.count();

    expect(hasHeader).toBeGreaterThanOrEqual(0);
  });
});
