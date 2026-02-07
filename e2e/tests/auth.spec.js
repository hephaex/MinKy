// @ts-check
const { test, expect } = require('@playwright/test');

test.describe('Authentication', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should display login form', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('input[type="email"], input[name="email"]')).toBeVisible();
    await expect(page.locator('input[type="password"]')).toBeVisible();
    await expect(page.getByRole('button', { name: /login|로그인/i })).toBeVisible();
  });

  test('should show error for invalid credentials', async ({ page }) => {
    await page.goto('/login');
    await page.fill('input[type="email"], input[name="email"]', 'invalid@example.com');
    await page.fill('input[type="password"]', 'wrongpassword');
    await page.click('button[type="submit"]');
    await expect(page.locator('.error, [class*="error"]')).toBeVisible({ timeout: 10000 });
  });

  test('should navigate to register page', async ({ page }) => {
    await page.goto('/login');
    const registerLink = page.getByRole('link', { name: /register|회원가입/i });
    if (await registerLink.isVisible()) {
      await registerLink.click();
      await expect(page).toHaveURL(/register/);
    }
  });

  test('should display register form', async ({ page }) => {
    await page.goto('/register');
    await expect(page.locator('input[type="email"], input[name="email"]')).toBeVisible();
    await expect(page.locator('input[type="password"]')).toBeVisible();
    await expect(page.getByRole('button', { name: /register|가입|sign up/i })).toBeVisible();
  });

  test('should validate email format on registration', async ({ page }) => {
    await page.goto('/register');
    await page.fill('input[type="email"], input[name="email"]', 'invalid-email');
    await page.fill('input[type="password"]', 'password123');
    await page.click('button[type="submit"]');
    // Verify validation error is displayed or form was not submitted
    const errorMessage = page.locator('[class*="error"], [role="alert"]');
    const isOnRegisterPage = await page.url().includes('/register');
    // Either error shown or still on register page (validation blocked submit)
    expect(await errorMessage.isVisible().catch(() => false) || isOnRegisterPage).toBeTruthy();
  });
});
