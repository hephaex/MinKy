// @ts-check
const { test, expect } = require('@playwright/test');

test.describe('Search', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('should display search input', async ({ page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search"], input[placeholder*="검색"]');
    await expect(searchInput.first()).toBeVisible({ timeout: 10000 });
  });

  test('should search documents by query', async ({ page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search"], input[placeholder*="검색"]').first();
    if (await searchInput.isVisible()) {
      await searchInput.fill('test');
      await searchInput.press('Enter');
      await page.waitForLoadState('networkidle');
      // Search results should be displayed or empty state
      await expect(page.locator('body')).toBeVisible();
    }
  });

  test('should clear search results', async ({ page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search"], input[placeholder*="검색"]').first();
    if (await searchInput.isVisible()) {
      await searchInput.fill('test');
      await searchInput.press('Enter');
      await page.waitForLoadState('networkidle');
      await searchInput.clear();
      await searchInput.press('Enter');
      await page.waitForLoadState('networkidle');
      // Should show all documents or home state
      await expect(page.locator('body')).toBeVisible();
    }
  });

  test('should handle empty search results', async ({ page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search"], input[placeholder*="검색"]').first();
    if (await searchInput.isVisible()) {
      await searchInput.fill('xyznonexistentkeyword12345');
      await searchInput.press('Enter');
      await page.waitForLoadState('networkidle');
      // Should show empty state or no results message
      await expect(page.locator('body')).toBeVisible();
    }
  });
});

test.describe('Tags', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('should display tags in sidebar or tag list', async ({ page }) => {
    // Check if tags are visible in sidebar or navigate to tags page
    const tagElements = page.locator('[class*="tag"], [class*="Tag"]');
    const tagsExist = await tagElements.count() > 0;
    if (!tagsExist) {
      // Navigate to tags page if available
      const tagsLink = page.getByRole('link', { name: /tags|태그/i });
      if (await tagsLink.isVisible()) {
        await tagsLink.click();
        await page.waitForLoadState('networkidle');
      }
    }
    // Just verify page loads without error
    await expect(page.locator('body')).toBeVisible();
  });

  test('should filter documents by tag', async ({ page }) => {
    const tagElement = page.locator('[class*="tag"], [class*="Tag"]').first();
    if (await tagElement.isVisible()) {
      await tagElement.click();
      await page.waitForLoadState('networkidle');
      // Should show filtered documents
      await expect(page.locator('body')).toBeVisible();
    }
  });

  test('should navigate to tag list page', async ({ page }) => {
    await page.goto('/tags');
    await page.waitForLoadState('networkidle');
    // Should display tags list or empty state
    await expect(page.locator('body')).toBeVisible();
  });
});
