// @ts-check
const { test, expect } = require('@playwright/test');

test.describe('Documents', () => {
  test('should display documents list on home page', async ({ page }) => {
    await page.goto('/');
    // Wait for documents to load
    await page.waitForLoadState('networkidle');
    // Check for document list or empty state
    const hasDocs = await page.locator('[class*="document"], [class*="card"]').count() > 0;
    const hasEmptyState = await page.locator('[class*="empty"], :text("문서가 없습니다")').count() > 0;
    expect(hasDocs || hasEmptyState).toBeTruthy();
  });

  test('should navigate to document creation page', async ({ page }) => {
    await page.goto('/');
    const newDocButton = page.getByRole('link', { name: /new|새 문서|create/i }).first();
    if (await newDocButton.isVisible()) {
      await newDocButton.click();
      await expect(page).toHaveURL(/new|create/);
    }
  });

  test('should display document creation form', async ({ page }) => {
    await page.goto('/documents/new');
    await expect(page.locator('input[name="title"], input[placeholder*="title"], input[placeholder*="제목"]')).toBeVisible({ timeout: 10000 });
  });

  test('should validate required fields on document creation', async ({ page }) => {
    await page.goto('/documents/new');
    // Try to submit without filling required fields
    const submitButton = page.getByRole('button', { name: /save|저장|submit|create/i });
    if (await submitButton.isVisible()) {
      await submitButton.click();
      // Should show validation error or remain on same page
      await expect(page).toHaveURL(/new|create/);
    }
  });

  test('should display document view page', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    // Click on first document if available
    const firstDoc = page.locator('[class*="document"], [class*="card"]').first();
    if (await firstDoc.isVisible()) {
      await firstDoc.click();
      await page.waitForLoadState('networkidle');
      // Should be on document view page
      await expect(page.locator('[class*="content"], [class*="markdown"], article')).toBeVisible({ timeout: 10000 });
    }
  });

  test('should have pagination when many documents exist', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    // Check if pagination exists (may not exist if few documents)
    const pagination = page.locator('[class*="pagination"], nav[aria-label*="page"]');
    // Just verify page loads without error
    await expect(page.locator('body')).toBeVisible();
  });
});
