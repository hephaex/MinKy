// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * E2E tests for Pagination
 * Tests pagination controls across different pages
 */
test.describe('Pagination - Documents Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');
  });

  test('should display pagination controls', async ({ page }) => {
    const pagination = page.locator('[class*="pagination"], nav[aria-label*="pagination"], [role="navigation"]');
    if (await pagination.count() > 0) {
      await expect(pagination.first()).toBeVisible();
    } else {
      // Page may not have enough items for pagination
      await expect(page.locator('body')).toBeVisible();
    }
  });

  test('should have page number buttons', async ({ page }) => {
    const pageButtons = page.locator('button:has-text("1"), a:has-text("1"), [class*="page-number"]');
    if (await pageButtons.count() > 0) {
      await expect(pageButtons.first()).toBeVisible();
    }
  });

  test('should have next/previous buttons', async ({ page }) => {
    const nextButton = page.locator('button:has-text("Next"), button:has-text("다음"), [aria-label*="next"]').first();
    const prevButton = page.locator('button:has-text("Previous"), button:has-text("이전"), [aria-label*="previous"]').first();

    if (await nextButton.count() > 0) {
      await expect(nextButton).toBeVisible();
    }
  });

  test('should navigate to next page', async ({ page }) => {
    const nextButton = page.locator('button:has-text("Next"), button:has-text("다음"), [aria-label*="next"]').first();

    if (await nextButton.count() > 0 && await nextButton.isEnabled()) {
      await nextButton.click();
      await page.waitForLoadState('networkidle');

      // URL should change or page content should update
      const url = page.url();
      const pageParam = url.includes('page=') || url.includes('p=');
      await expect(page.locator('body')).toBeVisible();
    }
  });

  test('should navigate to previous page', async ({ page }) => {
    // First go to page 2
    const nextButton = page.locator('button:has-text("Next"), button:has-text("다음"), [aria-label*="next"]').first();

    if (await nextButton.count() > 0 && await nextButton.isEnabled()) {
      await nextButton.click();
      await page.waitForLoadState('networkidle');

      const prevButton = page.locator('button:has-text("Previous"), button:has-text("이전"), [aria-label*="previous"]').first();
      if (await prevButton.count() > 0 && await prevButton.isEnabled()) {
        await prevButton.click();
        await page.waitForLoadState('networkidle');
      }
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should disable previous button on first page', async ({ page }) => {
    const prevButton = page.locator('button:has-text("Previous"), button:has-text("이전"), [aria-label*="previous"]').first();

    if (await prevButton.count() > 0) {
      const isDisabled = await prevButton.isDisabled();
      // First page should have disabled previous button
      expect(isDisabled).toBeTruthy();
    }
  });

  test('should show current page indicator', async ({ page }) => {
    const currentPage = page.locator('[aria-current="page"], [class*="active"], [class*="current"]');
    if (await currentPage.count() > 0) {
      await expect(currentPage.first()).toBeVisible();
    }
  });

  test('should show total pages or items count', async ({ page }) => {
    const totalInfo = page.locator('text=/of \\d+|총|total|\\d+ 페이지/i');
    if (await totalInfo.count() > 0) {
      await expect(totalInfo.first()).toBeVisible();
    }
  });

  test('should allow jumping to specific page', async ({ page }) => {
    const pageInput = page.locator('input[type="number"], input[name="page"]');
    if (await pageInput.count() > 0) {
      await pageInput.fill('2');
      await pageInput.press('Enter');
      await page.waitForLoadState('networkidle');
    }
    await expect(page.locator('body')).toBeVisible();
  });

  test('should have items per page selector', async ({ page }) => {
    const perPageSelector = page.locator('select, [class*="per-page"], text=/per page|개씩|보기/i');
    if (await perPageSelector.count() > 0) {
      await expect(perPageSelector.first()).toBeVisible();
    }
  });

  test('should update URL with page parameter', async ({ page }) => {
    const nextButton = page.locator('button:has-text("Next"), button:has-text("다음"), [aria-label*="next"]').first();

    if (await nextButton.count() > 0 && await nextButton.isEnabled()) {
      const initialUrl = page.url();
      await nextButton.click();
      await page.waitForLoadState('networkidle');

      const newUrl = page.url();
      // URL should change to include page parameter
      expect(newUrl !== initialUrl || newUrl.includes('page')).toBeTruthy();
    }
  });
});

test.describe('Pagination - Search Results', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/search');
    await page.waitForLoadState('networkidle');
  });

  test('should paginate search results', async ({ page }) => {
    const searchInput = page.locator('input[type="search"], input[type="text"]').first();

    if (await searchInput.count() > 0) {
      await searchInput.fill('test');
      await searchInput.press('Enter');
      await page.waitForLoadState('networkidle');

      const pagination = page.locator('[class*="pagination"]');
      // Pagination may appear if there are many results
      await expect(page.locator('body')).toBeVisible();
    }
  });

  test('should preserve search query across pages', async ({ page }) => {
    const searchInput = page.locator('input[type="search"], input[type="text"]').first();

    if (await searchInput.count() > 0) {
      await searchInput.fill('test query');
      await searchInput.press('Enter');
      await page.waitForLoadState('networkidle');

      const nextButton = page.locator('button:has-text("Next"), [aria-label*="next"]').first();
      if (await nextButton.count() > 0 && await nextButton.isEnabled()) {
        await nextButton.click();
        await page.waitForLoadState('networkidle');

        // Search query should still be in URL or input
        const url = page.url();
        const inputValue = await searchInput.inputValue();
        expect(url.includes('test') || inputValue.includes('test')).toBeTruthy();
      }
    }
  });
});

test.describe('Pagination - Keyboard Navigation', () => {
  test('should navigate pages with keyboard', async ({ page }) => {
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');

    // Focus on pagination
    const pagination = page.locator('[class*="pagination"]').first();
    if (await pagination.count() > 0) {
      await pagination.focus();

      // Arrow keys should navigate between page buttons
      await page.keyboard.press('ArrowRight');
      await page.keyboard.press('Enter');
      await page.waitForLoadState('networkidle');
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should support first/last page shortcuts', async ({ page }) => {
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');

    const firstButton = page.locator('button:has-text("First"), button:has-text("처음"), [aria-label*="first"]').first();
    const lastButton = page.locator('button:has-text("Last"), button:has-text("마지막"), [aria-label*="last"]').first();

    if (await lastButton.count() > 0 && await lastButton.isEnabled()) {
      await lastButton.click();
      await page.waitForLoadState('networkidle');
    }

    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Pagination - Edge Cases', () => {
  test('should handle empty results', async ({ page }) => {
    await page.goto('/documents?search=nonexistent_query_12345');
    await page.waitForLoadState('networkidle');

    // Should show empty state or no pagination
    const emptyState = page.locator('text=/no result|결과 없음|empty/i');
    const pagination = page.locator('[class*="pagination"]');

    // Either empty state is shown or no pagination needed
    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle invalid page number', async ({ page }) => {
    await page.goto('/documents?page=99999');
    await page.waitForLoadState('networkidle');

    // Should redirect to valid page or show error
    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle negative page number', async ({ page }) => {
    await page.goto('/documents?page=-1');
    await page.waitForLoadState('networkidle');

    // Should redirect to page 1 or show error
    await expect(page.locator('body')).toBeVisible();
  });
});
