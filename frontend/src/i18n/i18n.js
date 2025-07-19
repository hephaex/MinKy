/**
 * Internationalization (i18n) system for Minky
 * Supports multiple languages with Korean as the primary focus
 */

import { useState, useEffect, createContext, useContext } from 'react';
import en from './locales/en.json';
import ko from './locales/ko.json';
import ja from './locales/ja.json';
import zh from './locales/zh.json';
import './LanguageSelector.css';

// Available languages
export const LANGUAGES = {
  en: { code: 'en', name: 'English', nativeName: 'English' },
  ko: { code: 'ko', name: 'Korean', nativeName: '한국어' },
  ja: { code: 'ja', name: 'Japanese', nativeName: '日本語' },
  zh: { code: 'zh', name: 'Chinese', nativeName: '中文' }
};

// Translation resources
const resources = {
  en,
  ko,
  ja,
  zh
};

// Default language
const DEFAULT_LANGUAGE = 'en';

// Language detection
const detectLanguage = () => {
  // Check localStorage first
  const savedLanguage = localStorage.getItem('minky_language');
  if (savedLanguage && LANGUAGES[savedLanguage]) {
    return savedLanguage;
  }

  // Check browser language
  const browserLanguage = navigator.language || navigator.userLanguage;
  
  // Map browser languages to supported languages
  const languageMap = {
    'ko': 'ko',
    'ko-KR': 'ko',
    'ko-KP': 'ko',
    'ja': 'ja',
    'ja-JP': 'ja',
    'zh': 'zh',
    'zh-CN': 'zh',
    'zh-TW': 'zh',
    'zh-HK': 'zh',
    'en': 'en',
    'en-US': 'en',
    'en-GB': 'en'
  };

  const detectedLanguage = languageMap[browserLanguage] || 
                          languageMap[browserLanguage.split('-')[0]] || 
                          DEFAULT_LANGUAGE;

  return detectedLanguage;
};

// Translation function
const translate = (key, params = {}, language = DEFAULT_LANGUAGE) => {
  const keys = key.split('.');
  let translation = resources[language] || resources[DEFAULT_LANGUAGE];

  // Navigate through nested keys
  for (const k of keys) {
    translation = translation?.[k];
    if (translation === undefined) {
      // Fallback to default language
      translation = resources[DEFAULT_LANGUAGE];
      for (const fallbackKey of keys) {
        translation = translation?.[fallbackKey];
        if (translation === undefined) {
          console.warn(`Translation missing for key: ${key}`);
          return key; // Return the key itself if translation is not found
        }
      }
      break;
    }
  }

  if (typeof translation !== 'string') {
    console.warn(`Translation for key "${key}" is not a string:`, translation);
    return key;
  }

  // Replace parameters in translation
  let result = translation;
  Object.keys(params).forEach(param => {
    const placeholder = `{{${param}}}`;
    result = result.replace(new RegExp(placeholder, 'g'), params[param]);
  });

  return result;
};

// Pluralization function
const pluralize = (key, count, params = {}, language = DEFAULT_LANGUAGE) => {
  const pluralKey = count === 1 ? `${key}.one` : `${key}.other`;
  return translate(pluralKey, { ...params, count }, language);
};

// Date and number formatting
const formatDate = (date, options = {}, language = DEFAULT_LANGUAGE) => {
  const locale = language === 'ko' ? 'ko-KR' : 
                language === 'ja' ? 'ja-JP' :
                language === 'zh' ? 'zh-CN' : 'en-US';
  
  return new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    ...options
  }).format(new Date(date));
};

const formatNumber = (number, options = {}, language = DEFAULT_LANGUAGE) => {
  const locale = language === 'ko' ? 'ko-KR' : 
                language === 'ja' ? 'ja-JP' :
                language === 'zh' ? 'zh-CN' : 'en-US';
  
  return new Intl.NumberFormat(locale, options).format(number);
};

// Relative time formatting
const formatRelativeTime = (date, language = DEFAULT_LANGUAGE) => {
  const locale = language === 'ko' ? 'ko-KR' : 
                language === 'ja' ? 'ja-JP' :
                language === 'zh' ? 'zh-CN' : 'en-US';
  
  const now = new Date();
  const targetDate = new Date(date);
  const diffInSeconds = Math.floor((now - targetDate) / 1000);

  if (diffInSeconds < 60) {
    return translate('common.time.just_now', {}, language);
  }

  const diffInMinutes = Math.floor(diffInSeconds / 60);
  if (diffInMinutes < 60) {
    return pluralize('common.time.minutes_ago', diffInMinutes, { count: diffInMinutes }, language);
  }

  const diffInHours = Math.floor(diffInMinutes / 60);
  if (diffInHours < 24) {
    return pluralize('common.time.hours_ago', diffInHours, { count: diffInHours }, language);
  }

  const diffInDays = Math.floor(diffInHours / 24);
  if (diffInDays < 30) {
    return pluralize('common.time.days_ago', diffInDays, { count: diffInDays }, language);
  }

  // For older dates, show formatted date
  return formatDate(date, {}, language);
};

// Context for i18n
const I18nContext = createContext();

// Hook for using i18n
export const useI18n = () => {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider');
  }
  return context;
};

// i18n Provider component
export const I18nProvider = ({ children }) => {
  const [language, setLanguage] = useState(detectLanguage());
  const [isRTL, setIsRTL] = useState(false);

  useEffect(() => {
    // Save language to localStorage
    localStorage.setItem('minky_language', language);
    
    // Set document language and direction
    document.documentElement.lang = language;
    
    // Set RTL for languages that require it (Arabic, Hebrew, etc.)
    const rtlLanguages = ['ar', 'he', 'fa'];
    const shouldBeRTL = rtlLanguages.includes(language);
    setIsRTL(shouldBeRTL);
    document.documentElement.dir = shouldBeRTL ? 'rtl' : 'ltr';
    
    // Add language class to body for CSS styling
    document.body.className = document.body.className.replace(/lang-\w+/, '');
    document.body.classList.add(`lang-${language}`);
  }, [language]);

  const changeLanguage = (newLanguage) => {
    if (LANGUAGES[newLanguage]) {
      setLanguage(newLanguage);
    }
  };

  const t = (key, params = {}) => translate(key, params, language);
  const tn = (key, count, params = {}) => pluralize(key, count, params, language);
  const tDate = (date, options = {}) => formatDate(date, options, language);
  const tNumber = (number, options = {}) => formatNumber(number, options, language);
  const tRelative = (date) => formatRelativeTime(date, language);

  const value = {
    language,
    languages: LANGUAGES,
    isRTL,
    changeLanguage,
    t,
    tn,
    tDate,
    tNumber,
    tRelative
  };

  return (
    <I18nContext.Provider value={value}>
      {children}
    </I18nContext.Provider>
  );
};

// Language selector component
export const LanguageSelector = ({ className = '', showFlags = true }) => {
  const { language, changeLanguage, languages, t } = useI18n();

  const languageFlags = {
    en: '🇺🇸',
    ko: '🇰🇷',
    ja: '🇯🇵',
    zh: '🇨🇳'
  };

  return (
    <div className={`language-selector ${className}`}>
      <select
        value={language}
        onChange={(e) => changeLanguage(e.target.value)}
        className="language-select"
        title={t('common.change_language')}
      >
        {Object.values(languages).map((lang) => (
          <option key={lang.code} value={lang.code}>
            {showFlags && languageFlags[lang.code]} {lang.nativeName}
          </option>
        ))}
      </select>
    </div>
  );
};

// Text direction utilities
export const getTextDirection = (language) => {
  const rtlLanguages = ['ar', 'he', 'fa'];
  return rtlLanguages.includes(language) ? 'rtl' : 'ltr';
};

// Language-specific CSS class helper
export const getLanguageClass = (language) => {
  return `lang-${language}`;
};

// Export translation functions for use outside React components
export { translate as t, pluralize as tn, formatDate as tDate, formatNumber as tNumber, formatRelativeTime as tRelative };

export default {
  I18nProvider,
  useI18n,
  LanguageSelector,
  LANGUAGES,
  translate,
  pluralize,
  formatDate,
  formatNumber,
  formatRelativeTime,
  getTextDirection,
  getLanguageClass
};