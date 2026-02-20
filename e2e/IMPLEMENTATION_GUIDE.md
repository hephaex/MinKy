# MinKy E2E Test Suite - Implementation Guide

## Overview

The MinKy E2E test suite has been expanded from 46 to 178 test cases across 11 spec files. This guide explains the new test files, their purpose, and how to use them.

## New Test Files

### 1. `documents-crud.spec.js` (11 tests)

**Purpose:** Test complete Create, Read, Update, Delete workflows for documents.

**Tests:**
- Create documents with title and content validation
- Display document metadata with proper timestamps
- Edit document title and content
- Delete documents with confirmation dialogs
- Form validation for required fields
- Document preview functionality
- Sorting by modification date

**Key Scenarios:**
```javascript
// Create new document
await page.goto('/documents/new')
await titleInput.fill('Test Document')
await contentInput.fill('Document content')
await submitButton.click()

// Edit existing document
await firstDoc.click()
await editButton.click()
await titleInput.clear()
await titleInput.fill('Updated Title')
await saveButton.click()

// Delete with confirmation
await deleteButton.click()
await confirmButton.click()
```

**When to Use:**
- Testing document creation form
- Verifying timestamps are accurate
- Ensuring edit functionality works
- Testing delete confirmation flows

---

### 2. `filtering-tags.spec.js` (18 tests)

**Purpose:** Test tag/category filtering and multi-select scenarios.

**Test Groups:**

#### Tag & Category Filtering (13 tests)
- Display tag filter options
- Single tag filtering
- Multiple tag filters (AND logic)
- Clear individual and all filters
- Filter as chips/badges
- Filter state persistence
- Filter count indicators
- Autocomplete when adding tags
- Remove tags from documents

#### Search with Filters (5 tests)
- Search within filtered results
- Combine search + tag filters
- Clear search preserves filters

**Key Scenarios:**
```javascript
// Filter by single tag
await tagElement.click()
// Verify results filtered

// Apply multiple filters
await tags.nth(0).click()
await tags.nth(1).click()
// Results should match both

// Clear filters
await clearButton.click()
// All documents shown again
```

**Common Issues to Watch:**
- Filter state not persisting on navigation
- Filter UI not clearing when manually removing tags
- Search combining wrong with filters

---

### 3. `responsive-layout.spec.js` (29 tests)

**Purpose:** Ensure layout works on 6 different viewport sizes (mobile, tablet, desktop).

**Tested Viewports:**
| Device | Width | Height |
|--------|-------|--------|
| iPhone SE | 375 | 667 |
| iPhone 12 | 390 | 844 |
| iPad Mini | 768 | 1024 |
| iPad Pro | 1024 | 1366 |
| Desktop Small | 1280 | 720 |
| Desktop Large | 1920 | 1080 |

**Test Categories:**

#### Layout Tests (12 tests)
- Home page at all breakpoints
- Document list responsive
- Cards stack vertically on mobile, grid on desktop
- Chat page responsiveness
- Forms full-width on mobile

#### Navigation Tests (6 tests)
- Hamburger menu on mobile
- Full nav on desktop
- Mobile menu toggle
- Navigation visibility at breakpoints

#### Form Tests (6 tests)
- Input field width
- Field stacking (vertical on mobile)
- Button sizing (44x44 minimum for touch)
- Button appearance at different sizes

#### Media Tests (2 tests)
- Image scaling
- Video embed responsiveness

#### Touch Interaction Tests (3 tests)
- Touch target size (44x44 minimum)
- No hover-only elements
- Smooth scrolling

**Key Assertions:**
```javascript
// Check content fits viewport
const bodyWidth = await page.evaluate(() => document.body.scrollWidth)
expect(bodyWidth).toBeLessThanOrEqual(viewport.width + 20)

// Verify card stacking on mobile
const firstBox = await firstCard.boundingBox()
const secondBox = await secondCard.boundingBox()
expect(secondBox.y).toBeGreaterThanOrEqual(firstBox.y) // Below

// Touch target minimum size
expect(box.height).toBeGreaterThanOrEqual(44)
expect(box.width).toBeGreaterThanOrEqual(44)
```

---

### 4. `accessibility.spec.js` (31 tests)

**Purpose:** Comprehensive accessibility testing for WCAG 2.1 Level A compliance.

**Test Categories:**

#### Keyboard Navigation (6 tests)
- Tab key through interactive elements
- Enter key for buttons
- Space key for buttons
- Escape key closes modals
- Skip-to-content links
- Focus order logic

#### ARIA Attributes (9 tests)
- Proper ARIA roles (main, navigation, banner)
- ARIA labels for icon buttons
- Form input labels
- Required field indicators
- Heading hierarchy (H1 should exist)
- aria-live regions
- aria-busy for loading
- aria-expanded for toggles

#### Focus Management (3 tests)
- Focus indicator visibility
- Focus return after modal close
- Focus trapping in modals

#### Semantic HTML (4 tests)
- Use of semantic elements
- Proper list structure
- Button elements for actions
- Form elements properly used

#### Mobile Accessibility (3 tests)
- Touch target sizing (44x44)
- Text zoom handling
- Readable font sizes

#### Color & Contrast (2 tests)
- Text contrast verification
- Link color visibility

**Key Test Pattern:**
```javascript
// Check ARIA roles
const main = page.locator('[role="main"], main')
const nav = page.locator('[role="navigation"], nav')
expect((await main.count() > 0) || (await nav.count() > 0)).toBeTruthy()

// Verify focus indicator
await page.keyboard.press('Tab')
const focused = await page.evaluate(() => document.activeElement?.tagName)
expect(['BUTTON', 'A', 'INPUT']).toContain(focused)

// Check touch targets
const box = await button.boundingBox()
expect(box.height).toBeGreaterThanOrEqual(44)
expect(box.width).toBeGreaterThanOrEqual(44)
```

---

### 5. `error-handling.spec.js` (35 tests)

**Purpose:** Test error states, validation, and graceful degradation.

**Test Categories:**

#### Form Validation (5 tests)
- Required field validation
- Email format validation
- Password length requirements
- Error clearing on correction
- Multiple validation errors

#### Network Resilience (6 tests)
- Slow network (3G simulation)
- Loading indicators
- Failed API requests
- Offline mode
- Request retry logic

#### Error States (4 tests)
- 404 page handling
- Document not found
- Empty states
- Empty search results

#### Timeout & Loading (3 tests)
- Slow page loads
- Loading state on submit
- Graceful degradation

#### Permission Errors (2 tests)
- Unauthorized (401)
- Forbidden (403)

#### Boundary Conditions (6 tests)
- Very long titles/content
- Special characters in search
- Rapid form submissions
- XSS prevention
- Large data handling

#### Data Integrity (4 tests)
- Preserve form data on validation error
- Don't lose data on navigation
- No duplicate submissions
- Form state recovery

**Network Simulation Examples:**
```javascript
// Simulate slow network
await page.route('**/*', (route) => {
  setTimeout(() => route.continue(), 2000)
})

// Simulate failed API
await page.route('**/api/**', (route) => route.abort())

// Simulate offline
await page.context().setOffline(true)

// Conditional failure (first fails, second succeeds)
let requestCount = 0
await page.route('**/api/**', (route) => {
  if (requestCount++ === 0) {
    route.abort()
  } else {
    route.continue()
  }
})
```

---

## Existing Test Files (Updated)

### Original Files with Enhancement Notes

#### `auth.spec.js` (6 tests)
- Login form display and validation
- Register form and validation
- Email format checking
- Password requirements

#### `chat.spec.js` (9 tests)
- Chat interface rendering
- Message input
- Send button functionality
- Session management
- Empty state display
- Accessibility (ARIA roles)

#### `documents.spec.js` (7 tests)
- Document list display
- Document creation navigation
- Document viewing
- Pagination handling
- Empty state display

#### `knowledge.spec.js` (12 tests)
- Knowledge search page
- Semantic search capability
- Knowledge graph visualization
- Graph filters
- Node types display
- Empty graph handling

#### `navigation.spec.js` (11 tests)
- Home page loading
- Header navigation
- Settings page access
- Documents list navigation
- Chat page access
- Knowledge search access
- Graph page access
- Mobile sidebar toggle
- Tablet responsiveness
- 404 page handling

#### `search.spec.js` (9 tests)
- Search input visibility
- Document search by keyword
- Search result clearing
- Empty search results
- Tag display in sidebar
- Filter by tag
- Tag list navigation

---

## Best Practices Implemented

### 1. Flexible Selectors
```javascript
// Works without data-testid attributes
const titleInput = page.locator('input[name="title"], input[placeholder*="title"], input[placeholder*="제목"]')
const submitButton = page.getByRole('button', { name: /save|저장|create|submit/i })
```

**Why:** Tests work even if UI structure changes, supports multiple languages.

### 2. Graceful Feature Detection
```javascript
// Optional features don't fail tests
if (await someFeature.count() > 0) {
  await someFeature.click()
}
// Test continues if feature doesn't exist
```

**Why:** Works with feature flags, experimental UI, conditional rendering.

### 3. Proper Wait Strategies
```javascript
// Wait for network
await page.waitForLoadState('networkidle')

// Wait for API response
await page.waitForResponse(resp => resp.url().includes('/api/search'))

// Wait for element
await page.locator('[class*="loader"]').waitFor({ state: 'hidden' })
```

**Why:** Prevents flaky tests from race conditions.

### 4. AAA Pattern
```javascript
// Arrange - setup
await page.goto('/documents/new')

// Act - perform action
await titleInput.fill('Test')
await submitButton.click()

// Assert - verify result
await expect(page).toHaveURL(/documents/)
```

**Why:** Clear test intent, easier debugging.

### 5. Network Simulation
```javascript
// Test error handling
await page.route('**/api/**', (route) => route.abort())
await page.goto('/documents')
// Verify graceful error handling
```

**Why:** Test offline/slow network scenarios without infrastructure.

---

## Running Tests

### Quick Start
```bash
cd /Users/mare/Simon/minky/e2e

# Install dependencies (if needed)
npm install

# Run all tests
npx playwright test

# Run specific file
npx playwright test tests/accessibility.spec.js

# Run with UI (interactive)
npx playwright test --ui

# Run with browser visible (headed mode)
npx playwright test --headed

# Debug specific test
npx playwright test tests/responsive-layout.spec.js --debug
```

### View Results
```bash
# HTML report (opens in browser)
npx playwright show-report

# JUnit XML (for CI)
cat playwright-results.xml

# JSON results
cat playwright-results.json
```

### Run by Category
```bash
# Accessibility only
npx playwright test --grep "Keyboard|ARIA|Focus|Semantic|Color"

# Mobile only
npx playwright test responsive-layout

# Error handling only
npx playwright test error-handling

# Specific browser
npx playwright test --project=chromium
npx playwright test --project="Mobile Chrome"
```

---

## Debugging Tips

### 1. See Test Steps
```bash
npx playwright test tests/filtering-tags.spec.js --debug
```
- Uses Playwright Inspector
- Step through test code
- See element tree
- Console access

### 2. Generate Traces
```javascript
// In playwright.config.js:
use: {
  trace: 'on-first-retry'  // Captures full trace on failure
}
```
- View with `npx playwright show-trace`
- See network requests, DOM changes, screenshots

### 3. Screenshots on Failure
```javascript
// In playwright.config.js:
use: {
  screenshot: 'only-on-failure'
}
```

### 4. Video Recording
```javascript
// In playwright.config.js:
use: {
  video: 'retain-on-failure'
}
```

### 5. Check Test Output
```bash
# Verbose output
npx playwright test --reporter=list

# Detailed reporter
npx playwright test --reporter=list -vv
```

---

## Common Issues & Solutions

### Issue: Tests Timeout
**Solution:**
```javascript
// Increase timeout in config
timeout: 120000  // 2 minutes

// Or per test
test.setTimeout(180000)

// Or specific action
await page.waitForLoadState('networkidle', { timeout: 30000 })
```

### Issue: Flaky Tests (Intermittent Failures)
**Solution:**
```javascript
// Use proper waits instead of sleep
await page.waitForLoadState('networkidle')
await page.waitForResponse(resp => resp.url().includes('/api'))

// Or add retry
test('flaky test', async ({ page }) => {
  for (let i = 0; i < 3; i++) {
    try {
      await expect(element).toBeVisible()
      break
    } catch (e) {
      if (i === 2) throw e
      await page.waitForTimeout(500)
    }
  }
})
```

### Issue: Element Not Found
**Solution:**
```javascript
// Use flexible selectors
const input = page.locator('input[name="email"], input[placeholder*="email"]')

// Check visibility first
if (await input.isVisible()) {
  await input.fill('test@example.com')
}

// Wait for element
await input.waitFor({ state: 'visible', timeout: 10000 })
```

### Issue: Tests Pass Locally But Fail in CI
**Solution:**
```javascript
// Use absolute waits
await page.waitForLoadState('networkidle')
await page.waitForLoadState('domcontentloaded')

// Increase CI timeouts
workers: 1  // Serial execution
retries: 2  // Retry on failure
timeout: 120000  // Longer overall timeout
```

---

## Maintenance Guidelines

### Adding New Tests

1. **Choose Right File:**
   - New document feature → `documents-crud.spec.js`
   - New filter type → `filtering-tags.spec.js`
   - New error scenario → `error-handling.spec.js`
   - Accessibility concern → `accessibility.spec.js`
   - Mobile layout → `responsive-layout.spec.js`
   - New page → create new file

2. **Follow Pattern:**
```javascript
test('should describe exact behavior', async ({ page }) => {
  // Arrange - setup state
  await page.goto('/path')
  await page.waitForLoadState('networkidle')

  // Act - perform action
  await element.click()

  // Assert - verify result
  await expect(page).toHaveURL(/expected/)
})
```

3. **Test Both Happy & Sad Paths:**
```javascript
test('should work normally', async ({ page }) => {
  // Happy path
})

test('should show error on failure', async ({ page }) => {
  // Error path
})

test('should handle edge case', async ({ page }) => {
  // Edge case
})
```

### Updating Tests

When UI changes:
1. Update selectors to be flexible
2. Use `getByRole()` or `getByLabel()` when possible
3. Add feature detection for optional elements
4. Don't hardcode test data

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: E2E Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: 18
      - run: npm ci --prefix e2e
      - run: npx playwright install --with-deps
      - run: npx playwright test
        env:
          BASE_URL: http://localhost:3000
      - uses: actions/upload-artifact@v3
        if: always()
        with:
          name: playwright-report
          path: e2e/playwright-report/
```

---

## Metrics & Reporting

### Test Coverage Goals
- **Overall:** 80%+ pass rate
- **Critical flows:** 95%+
- **Accessibility:** 100% (standards-based)
- **Mobile layouts:** All major breakpoints

### Monitoring
- Track test execution time
- Monitor flaky test rate
- Alert on new failures
- Track coverage over time

---

## References

- [Playwright Docs](https://playwright.dev)
- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [Accessibility Testing](https://www.w3.org/WAI/test-evaluate/)
- [Best Practices](https://playwright.dev/docs/best-practices)

---

**Last Updated:** 2026-02-19
**Test Suite Version:** 2.0
**Total Tests:** 178
