import React from 'react';
import { render } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { I18nProvider } from './i18n/i18n';

/**
 * Custom render function that wraps components with necessary providers
 */
const AllTheProviders = ({ children }) => {
  return (
    <BrowserRouter>
      <I18nProvider>
        {children}
      </I18nProvider>
    </BrowserRouter>
  );
};

const customRender = (ui, options) =>
  render(ui, { wrapper: AllTheProviders, ...options });

// Re-export everything
export * from '@testing-library/react';

// Override render method
export { customRender as render };

/**
 * Mock I18n context for testing
 */
export const mockI18n = {
  language: 'en',
  languages: {
    en: { code: 'en', name: 'English', nativeName: 'English' },
    ko: { code: 'ko', name: 'Korean', nativeName: '한국어' },
  },
  isRTL: false,
  changeLanguage: jest.fn(),
  t: (key) => key,
  tn: (key, count) => `${key}.${count === 1 ? 'one' : 'other'}`,
  tDate: (date) => new Date(date).toLocaleDateString(),
  tNumber: (num) => num.toString(),
  tRelative: (date) => 'recently',
};

/**
 * Create a mock I18n provider for testing
 */
export const MockI18nProvider = ({ children, value = mockI18n }) => {
  const I18nContext = React.createContext(value);
  return (
    <I18nContext.Provider value={value}>
      {children}
    </I18nContext.Provider>
  );
};
