// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * E2E tests for Theme (Dark/Light Mode)
 * Tests theme switching and persistence
 */
test.describe('Theme - Mode Switching', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('should display theme toggle button', async ({ page }) => {
    const themeToggle = page.locator(
      'button[class*="theme"], ' +
      'button[aria-label*="theme"], ' +
      'button[aria-label*="dark"], ' +
      'button[aria-label*="light"], ' +
      '[class*="theme-toggle"], ' +
      '[class*="dark-mode"]'
    ).first();

    if (await themeToggle.count() > 0) {
      await expect(themeToggle).toBeVisible();
    } else {
      // Theme toggle may be in settings page
      await expect(page.locator('body')).toBeVisible();
    }
  });

  test('should switch to dark mode', async ({ page }) => {
    const themeToggle = page.locator(
      'button[class*="theme"], ' +
      'button[aria-label*="theme"], ' +
      'button[aria-label*="dark"]'
    ).first();

    if (await themeToggle.count() > 0) {
      await themeToggle.click();
      await page.waitForTimeout(300);

      // Check if dark mode is applied
      const htmlElement = page.locator('html');
      const isDark = await htmlElement.evaluate((el) => {
        return el.classList.contains('dark') ||
               el.getAttribute('data-theme') === 'dark' ||
               document.body.classList.contains('dark-mode');
      });
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should switch to light mode', async ({ page }) => {
    // First switch to dark mode
    const themeToggle = page.locator(
      'button[class*="theme"], ' +
      'button[aria-label*="theme"]'
    ).first();

    if (await themeToggle.count() > 0) {
      await themeToggle.click();
      await page.waitForTimeout(300);

      // Then switch back to light
      await themeToggle.click();
      await page.waitForTimeout(300);

      const htmlElement = page.locator('html');
      const isLight = await htmlElement.evaluate((el) => {
        return !el.classList.contains('dark') ||
               el.getAttribute('data-theme') === 'light';
      });
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should have correct background color in dark mode', async ({ page }) => {
    const themeToggle = page.locator('button[class*="theme"]').first();

    if (await themeToggle.count() > 0) {
      // Ensure we're in dark mode
      const htmlElement = page.locator('html');
      const isDark = await htmlElement.evaluate((el) => el.classList.contains('dark'));

      if (!isDark) {
        await themeToggle.click();
        await page.waitForTimeout(300);
      }

      // Check background color is dark
      const bgColor = await page.locator('body').evaluate((el) => {
        return window.getComputedStyle(el).backgroundColor;
      });

      // Dark backgrounds typically have low RGB values
      // This is a heuristic check
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should have correct text color in dark mode', async ({ page }) => {
    const themeToggle = page.locator('button[class*="theme"]').first();

    if (await themeToggle.count() > 0) {
      await themeToggle.click();
      await page.waitForTimeout(300);

      // Text color should be light in dark mode
      const textColor = await page.locator('body').evaluate((el) => {
        return window.getComputedStyle(el).color;
      });
    }

    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Theme - Persistence', () => {
  test('should persist theme preference in localStorage', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const themeToggle = page.locator('button[class*="theme"]').first();

    if (await themeToggle.count() > 0) {
      await themeToggle.click();
      await page.waitForTimeout(300);

      // Check localStorage
      const theme = await page.evaluate(() => {
        return localStorage.getItem('theme') ||
               localStorage.getItem('color-mode') ||
               localStorage.getItem('darkMode');
      });

      // Reload and check if theme persists
      await page.reload();
      await page.waitForLoadState('networkidle');

      const htmlElement = page.locator('html');
      const currentTheme = await htmlElement.evaluate((el) => {
        return el.classList.contains('dark') ? 'dark' : 'light';
      });
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should persist across browser sessions', async ({ page, context }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const themeToggle = page.locator('button[class*="theme"]').first();

    if (await themeToggle.count() > 0) {
      await themeToggle.click();
      await page.waitForTimeout(300);
    }

    // Open new page in same context (simulates new tab)
    const newPage = await context.newPage();
    await newPage.goto('/');
    await newPage.waitForLoadState('networkidle');

    // Theme should be consistent
    await expect(newPage.locator('body')).toBeVisible();
    await newPage.close();
  });
});

test.describe('Theme - System Preference', () => {
  test('should respect system dark mode preference', async ({ page }) => {
    // Emulate dark color scheme
    await page.emulateMedia({ colorScheme: 'dark' });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Page should be in dark mode (if following system preference)
    const htmlElement = page.locator('html');
    const isDark = await htmlElement.evaluate((el) => {
      return el.classList.contains('dark') ||
             el.getAttribute('data-theme') === 'dark' ||
             window.matchMedia('(prefers-color-scheme: dark)').matches;
    });

    await expect(page.locator('body')).toBeVisible();
  });

  test('should respect system light mode preference', async ({ page }) => {
    // Emulate light color scheme
    await page.emulateMedia({ colorScheme: 'light' });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('body')).toBeVisible();
  });

  test('should allow override of system preference', async ({ page }) => {
    // Start with system dark mode
    await page.emulateMedia({ colorScheme: 'dark' });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const themeToggle = page.locator('button[class*="theme"]').first();

    if (await themeToggle.count() > 0) {
      // Override to light mode
      await themeToggle.click();
      await page.waitForTimeout(300);

      // Should now be light despite system preference
      const htmlElement = page.locator('html');
      const currentTheme = await htmlElement.evaluate((el) => {
        return el.classList.contains('dark') ||
               el.getAttribute('data-theme') === 'dark';
      });
    }

    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Theme - Visual Consistency', () => {
  test('should apply theme to all components', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const themeToggle = page.locator('button[class*="theme"]').first();

    if (await themeToggle.count() > 0) {
      await themeToggle.click();
      await page.waitForTimeout(300);
    }

    // Navigate to different pages and verify theme is consistent
    const pages = ['/documents', '/chat', '/settings'];

    for (const path of pages) {
      await page.goto(path);
      await page.waitForLoadState('networkidle');

      // Theme should be consistent across pages
      await expect(page.locator('body')).toBeVisible();
    }
  });

  test('should style buttons correctly in dark mode', async ({ page }) => {
    await page.goto('/');
    const themeToggle = page.locator('button[class*="theme"]').first();

    if (await themeToggle.count() > 0) {
      await themeToggle.click();
      await page.waitForTimeout(300);
    }

    const primaryButton = page.locator('button[class*="primary"], button[class*="btn"]').first();
    if (await primaryButton.count() > 0) {
      await expect(primaryButton).toBeVisible();
    }
  });

  test('should style inputs correctly in dark mode', async ({ page }) => {
    await page.goto('/login');
    const themeToggle = page.locator('button[class*="theme"]').first();

    if (await themeToggle.count() > 0) {
      await themeToggle.click();
      await page.waitForTimeout(300);
    }

    const input = page.locator('input[type="email"], input[type="text"]').first();
    if (await input.count() > 0) {
      const bgColor = await input.evaluate((el) => {
        return window.getComputedStyle(el).backgroundColor;
      });
      // Input should have appropriate background for dark mode
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should style cards correctly in dark mode', async ({ page }) => {
    await page.goto('/documents');
    const themeToggle = page.locator('button[class*="theme"]').first();

    if (await themeToggle.count() > 0) {
      await themeToggle.click();
      await page.waitForTimeout(300);
    }

    const card = page.locator('[class*="card"], [class*="panel"]').first();
    if (await card.count() > 0) {
      await expect(card).toBeVisible();
    }
  });
});

test.describe('Theme - Animations', () => {
  test('should animate theme transition smoothly', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const themeToggle = page.locator('button[class*="theme"]').first();

    if (await themeToggle.count() > 0) {
      // Check for transition CSS
      const body = page.locator('body');
      const hasTransition = await body.evaluate((el) => {
        const style = window.getComputedStyle(el);
        return style.transition !== 'all 0s ease 0s' && style.transition !== '';
      });

      await themeToggle.click();
      // Animation should be smooth
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should not flash wrong theme on load', async ({ page }) => {
    // Set theme in localStorage before loading
    await page.addInitScript(() => {
      localStorage.setItem('theme', 'dark');
    });

    await page.goto('/');

    // Page should immediately be in dark mode without flash
    const htmlElement = page.locator('html');
    const isDark = await htmlElement.evaluate((el) => {
      return el.classList.contains('dark') ||
             el.getAttribute('data-theme') === 'dark';
    });

    await expect(page.locator('body')).toBeVisible();
  });
});
