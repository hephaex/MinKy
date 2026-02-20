# MinKy E2E Test Suite

## Quick Start

```bash
# Install dependencies
npm install

# Run all tests
npx playwright test

# Run with UI (interactive)
npx playwright test --ui

# View results
npx playwright show-report
```

## Test Suite Stats

- **Total Tests:** 178
- **Test Files:** 11
- **Coverage:** Authentication, CRUD, Search, Filtering, Responsive Design, Accessibility, Error Handling
- **Browsers:** Chromium, Firefox, WebKit, Mobile Chrome

## Test Files Overview

| File | Tests | Focus |
|------|-------|-------|
| `accessibility.spec.js` | 31 | A11y, keyboard nav, ARIA, focus |
| `responsive-layout.spec.js` | 29 | Mobile, tablet, desktop layouts |
| `error-handling.spec.js` | 35 | Form validation, network errors, edge cases |
| `filtering-tags.spec.js` | 18 | Tag/category filters, multi-select |
| `documents-crud.spec.js` | 11 | Create, read, update, delete documents |
| `knowledge.spec.js` | 12 | Knowledge search, graph, semantic |
| `navigation.spec.js` | 11 | Page routing, navigation UI |
| `auth.spec.js` | 6 | Login, register, validation |
| `chat.spec.js` | 9 | Chat interface, messaging |
| `search.spec.js` | 9 | Document search, empty results |
| `documents.spec.js` | 7 | Document listing, creation flow |

## Key Features

### Comprehensive Coverage
- Document CRUD operations
- Tag and category filtering
- Search functionality
- Authentication flows
- Chat interface
- Knowledge graph
- Navigation and routing

### Responsive Testing
Tests 6 viewport sizes:
- Mobile: 375px (iPhone SE), 390px (iPhone 12)
- Tablet: 768px (iPad Mini), 1024px (iPad Pro)
- Desktop: 1280px, 1920px

### Accessibility Testing
- Keyboard navigation
- ARIA attributes
- Color contrast
- Focus management
- Semantic HTML
- Screen reader support

### Error Handling
- Form validation
- Network failures
- 404 pages
- Empty states
- Timeouts
- Data integrity

## Common Commands

```bash
# Run all tests
npx playwright test

# Run specific file
npx playwright test tests/accessibility.spec.js

# Run tests matching pattern
npx playwright test --grep "mobile"

# Run single test
npx playwright test tests/accessibility.spec.js:45

# Debug mode
npx playwright test --debug

# Headed mode (see browser)
npx playwright test --headed

# Single browser
npx playwright test --project=chromium
npx playwright test --project="Mobile Chrome"

# Generate report
npx playwright show-report

# Update snapshots
npx playwright test --update-snapshots
```

## Configuration

### Config File
`playwright.config.js` - Playwright configuration
- Base URL: `http://localhost:3000`
- Timeout: 60 seconds per test
- Action timeout: 15 seconds
- Screenshot: only on failure
- Video: on first retry
- Retries: 2 on CI, 0 locally

### Server Setup
Automatically starts:
1. Frontend: `npm run start --prefix ../frontend`
2. Backend: `cargo run --release` in `../minky-rust/`

## Writing New Tests

### Basic Template
```javascript
test('should describe behavior', async ({ page }) => {
  // Arrange - setup
  await page.goto('/path')
  await page.waitForLoadState('networkidle')

  // Act - perform action
  await element.click()

  // Assert - verify
  await expect(page).toHaveURL(/expected/)
})
```

### Flexible Selectors
```javascript
// Works with multiple selector patterns
const input = page.locator(
  'input[name="title"], ' +
  'input[placeholder*="title"], ' +
  'input[placeholder*="제목"]'
)

// Or use getByRole
const button = page.getByRole('button', { name: /save|저장/i })
```

### Network Simulation
```javascript
// Slow network
await page.route('**/*', (route) => {
  setTimeout(() => route.continue(), 2000)
})

// Failed request
await page.route('**/api/**', (route) => route.abort())

// Offline
await page.context().setOffline(true)
```

## Debugging

### Inspector
```bash
npx playwright test --debug
```
- Pause on each test step
- Inspect element tree
- Execute JS in console

### Traces
```bash
# Auto-captured on failure
npx playwright show-trace trace.zip
```

### Screenshots
Auto-captured on failure in `test-results/`

### Logs
```bash
# Verbose output
npx playwright test --reporter=list -vv
```

## CI/CD

### GitHub Actions
Add to `.github/workflows/e2e.yml`:
```yaml
- run: npx playwright install --with-deps
- run: npx playwright test
  env:
    BASE_URL: http://localhost:3000
- uses: actions/upload-artifact@v3
  with:
    name: playwright-report
    path: playwright-report/
```

## Troubleshooting

### Tests Timeout
Increase timeout in config:
```javascript
timeout: 120000  // 2 minutes
```

### Flaky Tests
Use proper waits:
```javascript
// Good
await page.waitForLoadState('networkidle')
await page.waitForResponse(resp => resp.url().includes('/api'))

// Bad
await page.waitForTimeout(5000)  // ❌ Avoid arbitrary delays
```

### Element Not Found
Use flexible selectors:
```javascript
const input = page.locator('input[name="email"], input[placeholder*="email"]')

if (await input.isVisible()) {
  await input.fill('test@example.com')
}
```

## Documentation

- `TEST_SUMMARY.md` - Overview and statistics
- `IMPLEMENTATION_GUIDE.md` - Detailed guide for each test file
- `playwright.config.js` - Configuration details

## Contributing

1. Write tests first (TDD)
2. Use flexible selectors
3. Follow AAA pattern (Arrange, Act, Assert)
4. Handle missing features gracefully
5. Include comments for complex tests
6. Test both happy and error paths

## Version

- **Current:** 2.0
- **Tests:** 178
- **Coverage:** Complete UI, accessibility, mobile, error handling
- **Updated:** 2026-02-19

---

For detailed information, see `IMPLEMENTATION_GUIDE.md`
