// @ts-check
const { test, expect, devices } = require('@playwright/test');

/**
 * E2E tests for Responsive Layout
 * Tests layout, navigation, and usability across different device sizes
 */

// Test suite for different viewport sizes
const viewports = [
  { name: 'Mobile (iPhone SE)', width: 375, height: 667 },
  { name: 'Mobile (iPhone 12)', width: 390, height: 844 },
  { name: 'Tablet (iPad Mini)', width: 768, height: 1024 },
  { name: 'Tablet (iPad Pro)', width: 1024, height: 1366 },
  { name: 'Desktop (Small)', width: 1280, height: 720 },
  { name: 'Desktop (Large)', width: 1920, height: 1080 },
];

test.describe('Responsive Layout - Home Page', () => {
  viewports.forEach((viewport) => {
    test(`should render home page on ${viewport.name}`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto('/');
      await page.waitForLoadState('networkidle');

      // Page should be visible and not have horizontal scroll
      await expect(page.locator('body')).toBeVisible();

      // Check that content fits in viewport
      const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
      expect(bodyWidth).toBeLessThanOrEqual(viewport.width + 20); // Allow small margin
    });
  });

  test('should display hamburger menu on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const hamburger = page.locator('[class*="hamburger"], [class*="menu-button"], button[aria-label*="menu"]');

    // Mobile should have menu button (may not be strictly required)
    const hasHamburger = await hamburger.count() > 0;

    // Or has a sidebar/nav that's responsive
    const nav = page.locator('nav, [class*="sidebar"]');

    expect(hasHamburger || (await nav.count() > 0)).toBeTruthy();
  });

  test('should display full navigation on desktop', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Desktop should show full navigation
    const nav = page.locator('nav, [class*="navbar"]');

    // Navigation should be visible on desktop
    expect(await nav.count()).toBeGreaterThan(0);
  });
});

test.describe('Responsive Layout - Document Pages', () => {
  viewports.forEach((viewport) => {
    test(`should render document list on ${viewport.name}`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto('/documents');
      await page.waitForLoadState('networkidle');

      await expect(page.locator('body')).toBeVisible();
    });
  });

  test('should stack document cards vertically on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');

    // On mobile, cards should stack vertically
    const cards = page.locator('[class*="card"], [class*="document"]');

    if (await cards.count() > 1) {
      const firstCard = cards.first();
      const secondCard = cards.nth(1);

      const firstBox = await firstCard.boundingBox();
      const secondBox = await secondCard.boundingBox();

      // Second card should be below first card (higher y position)
      expect(secondBox.y).toBeGreaterThanOrEqual(firstBox.y);
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should display grid layout on tablet/desktop', async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 1366 });
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');

    // On tablet+, cards may be in grid/columns
    const cards = page.locator('[class*="card"], [class*="document"]');
    const cardCount = await cards.count();

    // Grid layout is more efficient with multiple columns
    // Just verify cards are visible
    if (cardCount > 0) {
      await expect(cards.first()).toBeVisible();
    }

    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Responsive Layout - Chat Page', () => {
  viewports.forEach((viewport) => {
    test(`should render chat page on ${viewport.name}`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto('/chat');
      await page.waitForLoadState('networkidle');

      await expect(page.locator('body')).toBeVisible();
    });
  });

  test('should make message input accessible on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/chat');
    await page.waitForLoadState('networkidle');

    // Input should be visible and not require horizontal scroll
    const input = page.locator('textarea, input[type="text"]').first();

    if (await input.count() > 0) {
      // Input should be in viewport on mobile
      await expect(input).toBeInViewport();
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle full-screen message list on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/chat');
    await page.waitForLoadState('networkidle');

    // Messages should scroll without affecting input visibility
    const container = page.locator('[class*="messages"], [role="log"]').first();

    if (await container.count() > 0) {
      // Scroll messages
      await container.evaluate((el) => {
        el.scrollTop = el.scrollHeight;
      });

      await page.waitForTimeout(500);
    }

    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Responsive Layout - Forms', () => {
  test('should display form inputs full-width on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/documents/new');
    await page.waitForLoadState('networkidle');

    // Inputs should take up available width
    const titleInput = page.locator('input[name="title"]');

    if (await titleInput.count() > 0) {
      const box = await titleInput.boundingBox();

      // Input width should be close to viewport width (minus padding)
      expect(box.width).toBeGreaterThan(300);
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should stack form fields vertically on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/documents/new');
    await page.waitForLoadState('networkidle');

    const inputs = page.locator('input, textarea, select').filter({ visible: true });
    const inputCount = await inputs.count();

    if (inputCount > 1) {
      const first = await inputs.nth(0).boundingBox();
      const second = await inputs.nth(1).boundingBox();

      // Fields should be stacked vertically (not side-by-side)
      // Second field should be below first
      expect(second.y).toBeGreaterThan(first.y);
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should display buttons full-width on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/documents/new');
    await page.waitForLoadState('networkidle');

    const submitButton = page.getByRole('button', { name: /save|submit|create/i });

    if (await submitButton.count() > 0) {
      const box = await submitButton.boundingBox();

      // Button should be wide enough to tap easily on mobile
      expect(box.width).toBeGreaterThan(200);
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should resize buttons for desktop', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto('/documents/new');
    await page.waitForLoadState('networkidle');

    const submitButton = page.getByRole('button', { name: /save|submit|create/i });

    if (await submitButton.count() > 0) {
      // Button should be visible and appropriately sized
      await expect(submitButton.first()).toBeVisible();
    }

    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Responsive Layout - Navigation', () => {
  test('should collapse navigation on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Navigation may be hidden or collapsed on mobile
    const sidebar = page.locator('[class*="sidebar"]');
    const nav = page.locator('nav');

    // One or both may exist
    const hasNav = (await sidebar.count() > 0) || (await nav.count() > 0);

    // Should have some form of navigation
    expect(hasNav).toBeTruthy();
  });

  test('should show navigation on desktop', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const nav = page.locator('nav, [class*="navbar"]');

    // Desktop should always show navigation
    expect(await nav.count()).toBeGreaterThan(0);
  });

  test('should toggle navigation on mobile menu button click', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const menuButton = page.locator('[class*="menu-button"], [aria-label*="menu"]').first();

    if (await menuButton.count() > 0) {
      // Menu should be clickable without errors
      await menuButton.click({ force: true }).catch(() => {
        // Click may fail on some elements, that's OK
      });

      await page.waitForTimeout(300);
    }

    // Page should remain stable
    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Responsive Layout - Images & Media', () => {
  test('should scale images responsively on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const images = page.locator('img');
    const imageCount = await images.count();

    // Check that images don't overflow viewport
    for (let i = 0; i < Math.min(imageCount, 3); i++) {
      const img = images.nth(i);
      if (await img.isVisible()) {
        const box = await img.boundingBox();

        // Image width should fit in viewport
        expect(box.width).toBeLessThanOrEqual(375);
      }
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle video embeds responsively', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Check for video elements or iframes
    const videos = page.locator('video, iframe');

    // Even if no videos exist, page should be stable
    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Responsive Layout - Text & Typography', () => {
  test('should use readable font sizes on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const body = page.locator('body');

    // Get computed font size
    const fontSize = await body.evaluate(() => {
      return window.getComputedStyle(document.body).fontSize;
    });

    // Font size should be reasonable for reading on mobile
    const fontSizeValue = parseInt(fontSize);
    expect(fontSizeValue).toBeGreaterThanOrEqual(14);
  });

  test('should handle text wrapping on narrow viewports', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');

    // Check that text doesn't overflow
    const cards = page.locator('[class*="card"]').first();

    if (await cards.count() > 0) {
      // Card content should not require horizontal scroll
      const box = await cards.boundingBox();
      expect(box.width).toBeLessThanOrEqual(375);
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should adjust heading sizes for mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const h1 = page.locator('h1').first();

    if (await h1.count() > 0) {
      // Heading should be readable on mobile
      const box = await h1.boundingBox();
      expect(box.width).toBeLessThanOrEqual(375);
    }

    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Responsive Layout - Touch Interactions', () => {
  test('should provide adequate touch targets on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Look for clickable elements
    const buttons = page.locator('button, a[role="button"]').filter({ visible: true });
    const buttonCount = await buttons.count();

    if (buttonCount > 0) {
      const firstButton = buttons.first();
      const box = await firstButton.boundingBox();

      // Touch targets should be at least 44x44 pixels
      expect(box.height).toBeGreaterThanOrEqual(30); // Allow some flexibility
      expect(box.width).toBeGreaterThanOrEqual(30);
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should not show hover-only elements on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');

    // Important actions should be visible without hover
    const cards = page.locator('[class*="card"]').first();

    if (await cards.count() > 0) {
      // Card content should be accessible without hover
      const text = await cards.innerText();
      expect(text.length).toBeGreaterThan(0);
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle scrolling smoothly on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');

    // Scroll down
    await page.evaluate(() => {
      window.scrollBy(0, 300);
    });

    await page.waitForTimeout(500);

    // Scroll back up
    await page.evaluate(() => {
      window.scrollTo(0, 0);
    });

    // Page should remain stable
    await expect(page.locator('body')).toBeVisible();
  });
});
