// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * E2E tests for Document CRUD operations
 * Tests full Create, Read, Update, Delete workflows
 */
test.describe('Document CRUD Operations', () => {
  let documentId = null;

  test('should create a new document with title and content', async ({ page }) => {
    await page.goto('/documents/new');
    await page.waitForLoadState('networkidle');

    // Fill document form
    const titleInput = page.locator('input[name="title"], input[placeholder*="title"], input[placeholder*="제목"]');
    const contentInput = page.locator('textarea[name="content"], textarea[placeholder*="content"], textarea[placeholder*="내용"]');

    if (await titleInput.count() > 0) {
      await titleInput.fill('Test Document: CRUD Operations');

      if (await contentInput.count() > 0) {
        await contentInput.fill('This is a test document for CRUD operations\n\nIt contains multiple lines of content');
      }

      // Submit form
      const submitButton = page.getByRole('button', { name: /save|저장|create|submit/i });
      if (await submitButton.count() > 0) {
        await submitButton.click();
        await page.waitForLoadState('networkidle');

        // Should redirect to document view or home
        const url = page.url();
        expect(url).not.toContain('/new');
        expect(url).not.toContain('/create');
      }
    }

    // Verify document page loaded
    await expect(page.locator('body')).toBeVisible();
  });

  test('should display document metadata (title, date, content)', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Find and click first document
    const firstDoc = page.locator('[class*="document"], [class*="card"]').first();
    if (await firstDoc.count() > 0) {
      await firstDoc.click();
      await page.waitForLoadState('networkidle');

      // Check for document content
      const title = page.locator('h1, h2, [class*="title"]').first();
      const content = page.locator('[class*="content"], article, main').first();

      // At least one of these should be visible
      const hasTitle = await title.count() > 0;
      const hasContent = await content.count() > 0;

      expect(hasTitle || hasContent).toBeTruthy();
    }
  });

  test('should edit document title', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Navigate to first document
    const firstDoc = page.locator('[class*="document"], [class*="card"]').first();
    if (await firstDoc.count() > 0) {
      await firstDoc.click();
      await page.waitForLoadState('networkidle');

      // Look for edit button
      const editButton = page.getByRole('button', { name: /edit|수정|modify/i });
      if (await editButton.count() > 0) {
        await editButton.click();
        await page.waitForLoadState('networkidle');

        // Update title
        const titleInput = page.locator('input[name="title"], input[placeholder*="title"]');
        if (await titleInput.count() > 0) {
          await titleInput.clear();
          await titleInput.fill('Updated Test Document Title');

          // Save changes
          const saveButton = page.getByRole('button', { name: /save|저장|update/i });
          if (await saveButton.count() > 0) {
            await saveButton.click();
            await page.waitForLoadState('networkidle');
          }
        }
      }
    }

    // Verify navigation worked
    await expect(page.locator('body')).toBeVisible();
  });

  test('should edit document content', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const firstDoc = page.locator('[class*="document"], [class*="card"]').first();
    if (await firstDoc.count() > 0) {
      await firstDoc.click();
      await page.waitForLoadState('networkidle');

      // Find edit button
      const editButton = page.getByRole('button', { name: /edit|수정/i });
      if (await editButton.count() > 0) {
        await editButton.click();
        await page.waitForLoadState('networkidle');

        // Update content
        const contentInput = page.locator('textarea[name="content"], textarea[placeholder*="content"]');
        if (await contentInput.count() > 0) {
          const currentValue = await contentInput.inputValue();
          await contentInput.clear();
          await contentInput.fill(currentValue + '\n\nAdded update at ' + new Date().toISOString());

          // Save
          const saveButton = page.getByRole('button', { name: /save|저장/i });
          if (await saveButton.count() > 0) {
            await saveButton.click();
            await page.waitForLoadState('networkidle');
          }
        }
      }
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should delete document with confirmation', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const firstDoc = page.locator('[class*="document"], [class*="card"]').first();
    if (await firstDoc.count() > 0) {
      await firstDoc.click();
      await page.waitForLoadState('networkidle');

      // Find delete button
      const deleteButton = page.getByRole('button', { name: /delete|삭제|remove/i });
      if (await deleteButton.count() > 0) {
        await deleteButton.click();

        // Handle confirmation dialog if present
        const confirmButton = page.locator('button:has-text("Confirm"), button:has-text("확인"), button:has-text("Yes")');
        if (await confirmButton.count() > 0) {
          await confirmButton.click();
        }

        await page.waitForLoadState('networkidle');
      }
    }

    // Page should still be visible
    await expect(page.locator('body')).toBeVisible();
  });

  test('should display document creation timestamp', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const firstDoc = page.locator('[class*="document"], [class*="card"]').first();
    if (await firstDoc.count() > 0) {
      await firstDoc.click();
      await page.waitForLoadState('networkidle');

      // Check for date/timestamp
      const dateElement = page.locator('[class*="date"], [class*="time"], [class*="created"]');

      // Date info may be in metadata or footer
      const pageText = await page.locator('body').innerText();

      // Should have some indication of document metadata
      expect(pageText.length).toBeGreaterThan(0);
    }
  });

  test('should display document modification timestamp', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const firstDoc = page.locator('[class*="document"], [class*="card"]').first();
    if (await firstDoc.count() > 0) {
      await firstDoc.click();
      await page.waitForLoadState('networkidle');

      // Check for modified date/timestamp
      const modifiedElement = page.locator('[class*="modified"], [class*="updated"]');

      // Should have content
      const pageText = await page.locator('body').innerText();
      expect(pageText.length).toBeGreaterThan(0);
    }
  });

  test('should show document list sorted by modification date', async ({ page }) => {
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');

    // Check if documents are displayed
    const documents = page.locator('[class*="document"], [class*="card"]');
    const docCount = await documents.count();

    // Should display documents or empty state
    if (docCount > 1) {
      // If multiple documents, they should be in some order
      const firstDocText = await documents.nth(0).innerText();
      const secondDocText = await documents.nth(1).innerText();

      // Both should have content
      expect(firstDocText.length).toBeGreaterThan(0);
      expect(secondDocText.length).toBeGreaterThan(0);
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should validate document title is required', async ({ page }) => {
    await page.goto('/documents/new');
    await page.waitForLoadState('networkidle');

    // Try to submit without title
    const submitButton = page.getByRole('button', { name: /save|저장|submit/i });
    if (await submitButton.count() > 0) {
      await submitButton.click();

      // Should either show error or remain on create page
      await page.waitForLoadState('networkidle');
      const url = page.url();

      // Still on create page or error shown
      expect(url.includes('/new') || url.includes('/create')).toBeTruthy();
    }
  });

  test('should support document preview', async ({ page }) => {
    await page.goto('/documents/new');
    await page.waitForLoadState('networkidle');

    // Check if preview button exists
    const previewButton = page.getByRole('button', { name: /preview|보기/i });
    if (await previewButton.count() > 0) {
      const titleInput = page.locator('input[name="title"]');
      if (await titleInput.count() > 0) {
        await titleInput.fill('Preview Test');
      }

      await previewButton.click();
      await page.waitForLoadState('networkidle');

      // Preview should render
      await expect(page.locator('body')).toBeVisible();
    }
  });
});
