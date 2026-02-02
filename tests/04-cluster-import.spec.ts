import { test, expect } from '@playwright/test';

test.describe('Cluster Import Modal', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Open import modal
    await page.locator('button[title="Add Cluster"]').click();
    await expect(page.locator('text=Import Clusters')).toBeVisible();
  });

  test('should display modal with correct structure', async ({ page }) => {
    // Modal title
    await expect(page.locator('h2:has-text("Import Clusters")')).toBeVisible();

    // Close button
    await expect(page.locator('button:has(svg)').filter({ hasText: '' }).first()).toBeVisible();

    // Tabs
    await expect(page.locator('text=Import from File')).toBeVisible();
    await expect(page.locator('text=Import from Folder')).toBeVisible();
  });

  test('should have file import tab active by default', async ({ page }) => {
    // File tab should be active (has primary color)
    const fileTab = page.locator('button:has-text("Import from File")');
    await expect(fileTab).toHaveClass(/border-primary/);
    await expect(fileTab).toHaveClass(/text-primary/);
  });

  test('should switch to folder tab', async ({ page }) => {
    // Click folder tab
    await page.locator('button:has-text("Import from Folder")').click();

    // Folder tab should now be active
    const folderTab = page.locator('button:has-text("Import from Folder")');
    await expect(folderTab).toHaveClass(/border-primary/);
    await expect(folderTab).toHaveClass(/text-primary/);
  });

  test('should show select file button in file tab', async ({ page }) => {
    // Should have select file button
    const selectButton = page.locator('button:has-text("Select File")');
    await expect(selectButton).toBeVisible();
  });

  test('should show select folder button in folder tab', async ({ page }) => {
    // Switch to folder tab
    await page.locator('button:has-text("Import from Folder")').click();

    // Should have select folder button
    const selectButton = page.locator('button:has-text("Select Folder")');
    await expect(selectButton).toBeVisible();
  });

  test('should show descriptive text for file import', async ({ page }) => {
    await expect(page.locator('text=Select a kubeconfig file to import')).toBeVisible();
  });

  test('should show descriptive text for folder import', async ({ page }) => {
    // Switch to folder tab
    await page.locator('button:has-text("Import from Folder")').click();

    await expect(page.locator('text=Select a folder to scan for kubeconfig files')).toBeVisible();
  });

  test('should close modal on backdrop click', async ({ page }) => {
    // Click outside modal (on backdrop)
    await page.locator('div.fixed.inset-0.bg-black\\/50').click({ position: { x: 10, y: 10 } });

    // Modal should close
    await expect(page.locator('text=Import Clusters')).not.toBeVisible();
  });

  test('should close modal on close button click', async ({ page }) => {
    // Click close button
    await page.locator('h2:has-text("Import Clusters")').locator('..').locator('button').first().click();

    // Modal should close
    await expect(page.locator('text=Import Clusters')).not.toBeVisible();
  });

  test('should clear discovered contexts when switching tabs', async ({ page }) => {
    // Switch to folder tab
    await page.locator('button:has-text("Import from Folder")').click();

    // Switch back to file tab
    await page.locator('button:has-text("Import from File")').click();

    // Should still show select button (not context list)
    await expect(page.locator('button:has-text("Select File")').or(page.locator('text=Scanning...'))).toBeVisible();
  });
});
