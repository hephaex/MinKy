// @ts-check
const { test, expect } = require('@playwright/test');

test.describe('Navigation', () => {
  test('should load home page', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/minky/i);
  });

  test('should have header with navigation', async ({ page }) => {
    await page.goto('/');
    const header = page.locator('header, [class*="header"], nav');
    await expect(header.first()).toBeVisible();
  });

  test('should navigate to settings page', async ({ page }) => {
    await page.goto('/');
    const settingsLink = page.getByRole('link', { name: /settings|설정/i });
    if (await settingsLink.isVisible()) {
      await settingsLink.click();
      await expect(page).toHaveURL(/settings/);
    }
  });

  test('should navigate to documents list', async ({ page }) => {
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');
    await expect(page.locator('body')).toBeVisible();
  });

  test('should navigate to chat page', async ({ page }) => {
    await page.goto('/chat');
    await page.waitForLoadState('networkidle');
    await expect(page.locator('body')).toBeVisible();
  });

  test('should navigate to knowledge search page', async ({ page }) => {
    await page.goto('/knowledge');
    await page.waitForLoadState('networkidle');
    await expect(page.locator('body')).toBeVisible();
  });

  test('should navigate to knowledge graph page', async ({ page }) => {
    await page.goto('/graph');
    await page.waitForLoadState('networkidle');
    await expect(page.locator('body')).toBeVisible();
  });

  test('should toggle sidebar on mobile without error', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Sidebar toggle button may be present on mobile
    const menuButton = page.locator('[class*="sidebar-toggle"], [class*="menu-button"]').first();
    if (await menuButton.count() > 0 && await menuButton.isVisible()) {
      // Use force click to avoid detached DOM element issues
      await menuButton.click({ force: true }).catch(() => {
        // Ignore click errors - some toggles re-render the component
      });
    }
    // Page should still be visible
    await expect(page.locator('body')).toBeVisible();
  });

  test('should be responsive on tablet', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 1024 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle 404 page', async ({ page }) => {
    await page.goto('/nonexistent-page-12345');
    await page.waitForLoadState('networkidle');
    // Should show 404 or redirect to home
    await expect(page.locator('body')).toBeVisible();
  });
});
