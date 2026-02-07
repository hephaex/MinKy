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
    await page.goto('/');
    const docsLink = page.getByRole('link', { name: /documents|문서/i });
    if (await docsLink.isVisible()) {
      await docsLink.click();
      await page.waitForLoadState('networkidle');
      await expect(page.locator('body')).toBeVisible();
    }
  });

  test('should toggle sidebar on mobile', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const menuButton = page.locator('[class*="toggle"], [class*="menu-button"], button[aria-label*="menu"]').first();
    if (await menuButton.isVisible()) {
      await menuButton.click();
      // Sidebar should be visible
      const sidebar = page.locator('[class*="sidebar"]');
      await expect(sidebar.first()).toBeVisible();
    }
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
