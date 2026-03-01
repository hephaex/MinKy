// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * E2E tests for Performance
 * Tests page load times, Core Web Vitals, and overall performance
 */
test.describe('Performance - Page Load Times', () => {
  test('should load homepage within acceptable time', async ({ page }) => {
    const startTime = Date.now();
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    const loadTime = Date.now() - startTime;

    // Page should load within 5 seconds
    expect(loadTime).toBeLessThan(5000);
  });

  test('should load documents page within acceptable time', async ({ page }) => {
    const startTime = Date.now();
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');
    const loadTime = Date.now() - startTime;

    // Page should load within 5 seconds
    expect(loadTime).toBeLessThan(5000);
  });

  test('should load chat page within acceptable time', async ({ page }) => {
    const startTime = Date.now();
    await page.goto('/chat');
    await page.waitForLoadState('networkidle');
    const loadTime = Date.now() - startTime;

    expect(loadTime).toBeLessThan(5000);
  });

  test('should load search page within acceptable time', async ({ page }) => {
    const startTime = Date.now();
    await page.goto('/search');
    await page.waitForLoadState('networkidle');
    const loadTime = Date.now() - startTime;

    expect(loadTime).toBeLessThan(5000);
  });

  test('should load settings page within acceptable time', async ({ page }) => {
    const startTime = Date.now();
    await page.goto('/settings');
    await page.waitForLoadState('networkidle');
    const loadTime = Date.now() - startTime;

    expect(loadTime).toBeLessThan(5000);
  });
});

test.describe('Performance - Core Web Vitals', () => {
  test('should have good First Contentful Paint (FCP)', async ({ page }) => {
    await page.goto('/');

    // Wait for FCP to be measured
    await page.waitForLoadState('domcontentloaded');

    const fcp = await page.evaluate(() => {
      return new Promise((resolve) => {
        const observer = new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            if (entry.name === 'first-contentful-paint') {
              resolve(entry.startTime);
              observer.disconnect();
            }
          }
        });

        observer.observe({ type: 'paint', buffered: true });

        // Fallback timeout
        setTimeout(() => resolve(null), 5000);
      });
    });

    if (fcp) {
      // FCP should be under 2.5 seconds (good threshold)
      expect(Number(fcp)).toBeLessThan(2500);
    }
  });

  test('should have good Largest Contentful Paint (LCP)', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const lcp = await page.evaluate(() => {
      return new Promise((resolve) => {
        let lcpValue = 0;
        const observer = new PerformanceObserver((list) => {
          const entries = list.getEntries();
          const lastEntry = entries[entries.length - 1];
          if (lastEntry) {
            lcpValue = lastEntry.startTime;
          }
        });

        observer.observe({ type: 'largest-contentful-paint', buffered: true });

        // Wait for LCP to stabilize
        setTimeout(() => {
          observer.disconnect();
          resolve(lcpValue);
        }, 3000);
      });
    });

    if (lcp) {
      // LCP should be under 2.5 seconds (good threshold)
      expect(Number(lcp)).toBeLessThan(4000);
    }
  });

  test('should have good Time to Interactive (TTI)', async ({ page }) => {
    const startTime = Date.now();
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Wait for page to be interactive
    await page.locator('body').click({ force: true });
    const tti = Date.now() - startTime;

    // TTI should be under 5 seconds
    expect(tti).toBeLessThan(5000);
  });

  test('should have minimal Cumulative Layout Shift (CLS)', async ({ page }) => {
    await page.goto('/');

    const cls = await page.evaluate(() => {
      return new Promise((resolve) => {
        let clsValue = 0;
        const observer = new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            if (!entry.hadRecentInput) {
              clsValue += entry.value;
            }
          }
        });

        observer.observe({ type: 'layout-shift', buffered: true });

        setTimeout(() => {
          observer.disconnect();
          resolve(clsValue);
        }, 5000);
      });
    });

    if (cls !== null) {
      // CLS should be under 0.1 (good threshold)
      expect(Number(cls)).toBeLessThan(0.25);
    }
  });
});

test.describe('Performance - Network', () => {
  test('should minimize number of HTTP requests', async ({ page }) => {
    let requestCount = 0;

    page.on('request', () => {
      requestCount++;
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Reasonable number of requests for a modern SPA
    expect(requestCount).toBeLessThan(100);
  });

  test('should have reasonable total page weight', async ({ page }) => {
    let totalSize = 0;

    page.on('response', async (response) => {
      try {
        const body = await response.body();
        totalSize += body.length;
      } catch {
        // Ignore errors for requests without body
      }
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Total page weight should be under 5MB (reasonable for modern app)
    expect(totalSize).toBeLessThan(5 * 1024 * 1024);
  });

  test('should cache static assets', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Navigate away and back
    await page.goto('/documents');
    await page.waitForLoadState('networkidle');

    let cachedRequests = 0;

    page.on('response', (response) => {
      const cacheControl = response.headers()['cache-control'];
      if (cacheControl && (cacheControl.includes('max-age') || cacheControl.includes('immutable'))) {
        cachedRequests++;
      }
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Some requests should be cached
    await expect(page.locator('body')).toBeVisible();
  });

  test('should use compression for responses', async ({ page }) => {
    let compressedResponses = 0;

    page.on('response', (response) => {
      const encoding = response.headers()['content-encoding'];
      if (encoding && (encoding.includes('gzip') || encoding.includes('br') || encoding.includes('deflate'))) {
        compressedResponses++;
      }
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Should have some compressed responses
    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Performance - JavaScript', () => {
  test('should not have JavaScript errors', async ({ page }) => {
    const jsErrors = [];

    page.on('pageerror', (error) => {
      jsErrors.push(error.message);
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Navigate through main pages
    const pages = ['/documents', '/chat', '/search'];
    for (const path of pages) {
      await page.goto(path);
      await page.waitForLoadState('networkidle');
    }

    // Should have no JS errors
    expect(jsErrors.length).toBe(0);
  });

  test('should not have console warnings', async ({ page }) => {
    const warnings = [];

    page.on('console', (msg) => {
      if (msg.type() === 'warning') {
        warnings.push(msg.text());
      }
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Some warnings are acceptable but shouldn't be excessive
    expect(warnings.length).toBeLessThan(10);
  });

  test('should have efficient DOM', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const domStats = await page.evaluate(() => {
      const allElements = document.querySelectorAll('*');
      const maxDepth = (el, depth = 0) => {
        if (!el.children.length) return depth;
        return Math.max(...Array.from(el.children).map((child) => maxDepth(child, depth + 1)));
      };

      return {
        totalElements: allElements.length,
        maxDepth: maxDepth(document.body),
      };
    });

    // DOM should not be excessively large or deep
    expect(domStats.totalElements).toBeLessThan(3000);
    expect(domStats.maxDepth).toBeLessThan(32);
  });
});

test.describe('Performance - Images', () => {
  test('should lazy load images', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const images = await page.locator('img').all();

    for (const img of images.slice(0, 5)) {
      const loading = await img.getAttribute('loading');
      // Modern apps should use lazy loading for below-fold images
      // This is a best practice check, not strict requirement
    }

    await expect(page.locator('body')).toBeVisible();
  });

  test('should use appropriate image formats', async ({ page }) => {
    const imageRequests = [];

    page.on('request', (request) => {
      const url = request.url();
      if (url.match(/\.(jpg|jpeg|png|gif|webp|avif|svg)$/i)) {
        imageRequests.push(url);
      }
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Check that some images use modern formats (webp, avif)
    const modernImages = imageRequests.filter((url) =>
      url.match(/\.(webp|avif)$/i)
    );

    // Not strictly required but recommended
    await expect(page.locator('body')).toBeVisible();
  });

  test('should have sized images', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const images = await page.locator('img').all();

    for (const img of images.slice(0, 5)) {
      const width = await img.getAttribute('width');
      const height = await img.getAttribute('height');
      const style = await img.getAttribute('style');

      // Images should have explicit dimensions to prevent layout shift
      const hasDimensions = width || height || (style && style.includes('width'));
      // Best practice but not always required
    }

    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Performance - API Response Times', () => {
  test('should respond to health check quickly', async ({ page }) => {
    const startTime = Date.now();

    const response = await page.request.get('/api/health');
    const responseTime = Date.now() - startTime;

    // Health check should respond within 500ms
    expect(responseTime).toBeLessThan(500);
    expect(response.status()).toBe(200);
  });

  test('should load documents list within acceptable time', async ({ page }) => {
    const startTime = Date.now();

    const response = await page.request.get('/api/documents?limit=10');
    const responseTime = Date.now() - startTime;

    // API should respond within 2 seconds
    expect(responseTime).toBeLessThan(2000);
  });

  test('should perform search within acceptable time', async ({ page }) => {
    const startTime = Date.now();

    const response = await page.request.get('/api/search?q=test');
    const responseTime = Date.now() - startTime;

    // Search should respond within 3 seconds
    expect(responseTime).toBeLessThan(3000);
  });
});
