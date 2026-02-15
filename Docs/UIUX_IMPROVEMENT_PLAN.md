# MinKy: UI/UX Improvement Plan

**Version:** 1.0
**Date:** 2026-02-15
**Status:** Planning Phase

---

## 1. Current State Analysis

### 1.1 Frontend Stack

| 기술 | 버전 | 용도 |
|------|------|------|
| React | 18.x | UI Framework |
| React Router | 6.x | Routing |
| Axios | 1.x | HTTP Client |
| Socket.IO Client | 4.x | Real-time |
| CSS Modules | - | Styling |

### 1.2 Component Inventory

```
frontend/src/components/ (94 files)
├── Core UI (15)
│   ├── TreeSidebar.js
│   ├── SearchBar.js
│   ├── LoadingSpinner.js
│   ├── ErrorBoundary.js
│   └── ...
├── Document (20)
│   ├── DocumentEditor.js
│   ├── DocumentViewer.js
│   ├── DocumentList.js
│   └── ...
├── Admin (4)
├── ML Analytics (3)
├── OCR (4)
├── Settings (4)
└── Others (44)
```

---

## 2. Identified UX Pain Points

### 2.1 Critical Issues

| 문제 | 영향도 | 발생 위치 |
|------|--------|-----------|
| 느린 초기 로딩 | HIGH | 전체 앱 |
| 모바일 반응형 미지원 | HIGH | 전체 레이아웃 |
| 검색 UX 불편 | HIGH | SearchBar |
| 협업 편집 시각적 피드백 부족 | MEDIUM | DocumentEditor |
| 에러 메시지 불명확 | MEDIUM | 전체 폼 |

### 2.2 User Journey Friction Points

```
문서 생성 플로우:
[홈] → [새 문서] → [편집] → [태그 추가] → [저장]
       ↓           ↓         ↓
   버튼 위치     자동저장    태그 추천
   불명확       피드백 없음  느림

검색 플로우:
[검색창] → [입력] → [결과] → [문서 열기]
            ↓        ↓
        자동완성   필터링
        느림       UI 복잡
```

---

## 3. Improvement Roadmap

### Phase 1: Quick Wins (2주)

#### 3.1.1 로딩 성능 개선

```jsx
// Before: 모든 컴포넌트 즉시 로딩
import DocumentEditor from './DocumentEditor';
import MLAnalytics from './MLAnalytics';
import AdminPanel from './AdminPanel';

// After: 지연 로딩
const DocumentEditor = lazy(() => import('./DocumentEditor'));
const MLAnalytics = lazy(() => import('./MLAnalytics'));
const AdminPanel = lazy(() => import('./AdminPanel'));

// Suspense 래퍼
<Suspense fallback={<LoadingSkeleton />}>
  <DocumentEditor />
</Suspense>
```

#### 3.1.2 로딩 스켈레톤 추가

```jsx
// components/ui/Skeleton.jsx
export const DocumentSkeleton = () => (
  <div className="skeleton-container">
    <div className="skeleton-title" />
    <div className="skeleton-content">
      <div className="skeleton-line" />
      <div className="skeleton-line" />
      <div className="skeleton-line short" />
    </div>
  </div>
);

// CSS
.skeleton-line {
  height: 16px;
  background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite;
  border-radius: 4px;
  margin-bottom: 8px;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
```

#### 3.1.3 Toast 알림 시스템

```jsx
// hooks/useToast.js
import { create } from 'zustand';

export const useToastStore = create((set) => ({
  toasts: [],
  addToast: (toast) => set((state) => ({
    toasts: [...state.toasts, { id: Date.now(), ...toast }]
  })),
  removeToast: (id) => set((state) => ({
    toasts: state.toasts.filter(t => t.id !== id)
  })),
}));

// components/ui/Toast.jsx
export const Toast = ({ type, message, onClose }) => (
  <div className={`toast toast-${type}`}>
    <span className="toast-icon">{icons[type]}</span>
    <span className="toast-message">{message}</span>
    <button onClick={onClose} className="toast-close">×</button>
  </div>
);
```

### Phase 2: Mobile & Responsive (3주)

#### 3.2.1 반응형 레이아웃

```css
/* styles/responsive.css */

/* Mobile First */
.app-layout {
  display: flex;
  flex-direction: column;
}

.sidebar {
  width: 100%;
  height: auto;
  position: fixed;
  bottom: 0;
  z-index: 100;
}

.main-content {
  padding-bottom: 60px; /* Sidebar height */
}

/* Tablet (768px+) */
@media (min-width: 768px) {
  .app-layout {
    flex-direction: row;
  }

  .sidebar {
    width: 280px;
    height: 100vh;
    position: sticky;
    top: 0;
    bottom: auto;
  }

  .main-content {
    flex: 1;
    padding-bottom: 0;
  }
}

/* Desktop (1024px+) */
@media (min-width: 1024px) {
  .sidebar {
    width: 320px;
  }

  .main-content {
    max-width: 1200px;
    margin: 0 auto;
  }
}
```

#### 3.2.2 모바일 네비게이션

```jsx
// components/mobile/BottomNav.jsx
export const BottomNav = () => {
  const location = useLocation();

  return (
    <nav className="bottom-nav">
      <NavItem to="/" icon={<HomeIcon />} label="홈" active={location.pathname === '/'} />
      <NavItem to="/search" icon={<SearchIcon />} label="검색" />
      <NavItem to="/new" icon={<PlusIcon />} label="새 문서" primary />
      <NavItem to="/notifications" icon={<BellIcon />} label="알림" badge={3} />
      <NavItem to="/profile" icon={<UserIcon />} label="프로필" />
    </nav>
  );
};

// CSS
.bottom-nav {
  display: flex;
  justify-content: space-around;
  align-items: center;
  height: 60px;
  background: white;
  border-top: 1px solid #eee;
  box-shadow: 0 -2px 10px rgba(0,0,0,0.1);
}

@media (min-width: 768px) {
  .bottom-nav {
    display: none;
  }
}
```

#### 3.2.3 터치 제스처 지원

```jsx
// hooks/useSwipe.js
import { useState, useRef } from 'react';

export const useSwipe = (onSwipeLeft, onSwipeRight) => {
  const touchStart = useRef(null);

  const handleTouchStart = (e) => {
    touchStart.current = e.touches[0].clientX;
  };

  const handleTouchEnd = (e) => {
    if (!touchStart.current) return;

    const touchEnd = e.changedTouches[0].clientX;
    const diff = touchStart.current - touchEnd;

    if (Math.abs(diff) > 50) {
      if (diff > 0) onSwipeLeft?.();
      else onSwipeRight?.();
    }

    touchStart.current = null;
  };

  return { handleTouchStart, handleTouchEnd };
};

// Usage: Document navigation
const { handleTouchStart, handleTouchEnd } = useSwipe(
  () => navigateToNext(),
  () => navigateToPrevious()
);
```

### Phase 3: Search & Discovery (2주)

#### 3.3.1 고급 검색 UI

```jsx
// components/search/AdvancedSearch.jsx
export const AdvancedSearch = () => {
  const [filters, setFilters] = useState({
    query: '',
    tags: [],
    category: null,
    dateRange: null,
    author: null,
  });

  return (
    <div className="advanced-search">
      <div className="search-input-wrapper">
        <SearchIcon className="search-icon" />
        <input
          type="text"
          placeholder="문서 검색..."
          value={filters.query}
          onChange={(e) => setFilters({ ...filters, query: e.target.value })}
          className="search-input"
        />
        <button className="filter-toggle">
          <FilterIcon />
        </button>
      </div>

      <div className="search-filters">
        <TagSelector
          selected={filters.tags}
          onChange={(tags) => setFilters({ ...filters, tags })}
        />
        <CategoryDropdown
          value={filters.category}
          onChange={(category) => setFilters({ ...filters, category })}
        />
        <DateRangePicker
          value={filters.dateRange}
          onChange={(dateRange) => setFilters({ ...filters, dateRange })}
        />
      </div>

      <div className="search-suggestions">
        {/* AI 기반 검색 제안 */}
      </div>
    </div>
  );
};
```

#### 3.3.2 실시간 자동완성

```jsx
// hooks/useAutocomplete.js
import { useState, useEffect, useCallback } from 'react';
import { debounce } from 'lodash';

export const useAutocomplete = (fetchSuggestions, delay = 300) => {
  const [query, setQuery] = useState('');
  const [suggestions, setSuggestions] = useState([]);
  const [isLoading, setIsLoading] = useState(false);

  const debouncedFetch = useCallback(
    debounce(async (q) => {
      if (q.length < 2) {
        setSuggestions([]);
        return;
      }

      setIsLoading(true);
      try {
        const results = await fetchSuggestions(q);
        setSuggestions(results);
      } finally {
        setIsLoading(false);
      }
    }, delay),
    [fetchSuggestions, delay]
  );

  useEffect(() => {
    debouncedFetch(query);
    return () => debouncedFetch.cancel();
  }, [query, debouncedFetch]);

  return { query, setQuery, suggestions, isLoading };
};
```

### Phase 4: Editor Experience (3주)

#### 3.4.1 실시간 협업 시각화

```jsx
// components/editor/CollaboratorCursors.jsx
export const CollaboratorCursors = ({ collaborators }) => {
  return (
    <>
      {collaborators.map((collab) => (
        <div
          key={collab.id}
          className="collaborator-cursor"
          style={{
            top: collab.cursor.top,
            left: collab.cursor.left,
            '--cursor-color': collab.color,
          }}
        >
          <div className="cursor-caret" />
          <div className="cursor-label">{collab.name}</div>
        </div>
      ))}
    </>
  );
};

// CSS
.collaborator-cursor {
  position: absolute;
  pointer-events: none;
  z-index: 10;
}

.cursor-caret {
  width: 2px;
  height: 20px;
  background: var(--cursor-color);
  animation: blink 1s infinite;
}

.cursor-label {
  position: absolute;
  top: -20px;
  left: 0;
  padding: 2px 6px;
  background: var(--cursor-color);
  color: white;
  font-size: 12px;
  border-radius: 4px;
  white-space: nowrap;
}
```

#### 3.4.2 자동 저장 인디케이터

```jsx
// components/editor/AutoSaveIndicator.jsx
export const AutoSaveIndicator = ({ status }) => {
  const statusConfig = {
    saving: { icon: <SpinnerIcon />, text: '저장 중...', color: 'blue' },
    saved: { icon: <CheckIcon />, text: '저장됨', color: 'green' },
    error: { icon: <AlertIcon />, text: '저장 실패', color: 'red' },
    offline: { icon: <OfflineIcon />, text: '오프라인', color: 'gray' },
  };

  const config = statusConfig[status];

  return (
    <div className={`autosave-indicator status-${config.color}`}>
      {config.icon}
      <span>{config.text}</span>
    </div>
  );
};
```

#### 3.4.3 키보드 단축키

```jsx
// hooks/useKeyboardShortcuts.js
import { useEffect, useCallback } from 'react';

const shortcuts = {
  'ctrl+s': 'save',
  'ctrl+shift+s': 'saveAs',
  'ctrl+b': 'bold',
  'ctrl+i': 'italic',
  'ctrl+k': 'link',
  'ctrl+/': 'toggleComment',
  'ctrl+shift+p': 'commandPalette',
};

export const useKeyboardShortcuts = (handlers) => {
  const handleKeyDown = useCallback((e) => {
    const key = [
      e.ctrlKey && 'ctrl',
      e.shiftKey && 'shift',
      e.altKey && 'alt',
      e.key.toLowerCase(),
    ].filter(Boolean).join('+');

    const action = shortcuts[key];
    if (action && handlers[action]) {
      e.preventDefault();
      handlers[action]();
    }
  }, [handlers]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
};

// components/editor/ShortcutsHelp.jsx
export const ShortcutsHelp = () => (
  <div className="shortcuts-panel">
    <h3>키보드 단축키</h3>
    <table>
      <tbody>
        <tr><td><kbd>Ctrl</kbd>+<kbd>S</kbd></td><td>저장</td></tr>
        <tr><td><kbd>Ctrl</kbd>+<kbd>B</kbd></td><td>굵게</td></tr>
        <tr><td><kbd>Ctrl</kbd>+<kbd>I</kbd></td><td>기울임</td></tr>
        <tr><td><kbd>Ctrl</kbd>+<kbd>K</kbd></td><td>링크</td></tr>
        <tr><td><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>P</kbd></td><td>명령 팔레트</td></tr>
      </tbody>
    </table>
  </div>
);
```

### Phase 5: AI Integration UX (2주)

#### 3.5.1 AI 어시스턴트 패널

```jsx
// components/ai/AIAssistant.jsx
export const AIAssistant = ({ documentContent, selection }) => {
  const [suggestions, setSuggestions] = useState([]);
  const [isLoading, setIsLoading] = useState(false);

  const actions = [
    { id: 'improve', label: '문장 개선', icon: <EditIcon /> },
    { id: 'summarize', label: '요약하기', icon: <SummaryIcon /> },
    { id: 'expand', label: '내용 확장', icon: <ExpandIcon /> },
    { id: 'translate', label: '번역', icon: <TranslateIcon /> },
    { id: 'tags', label: '태그 추천', icon: <TagIcon /> },
  ];

  return (
    <div className="ai-assistant">
      <div className="ai-header">
        <SparkleIcon />
        <span>AI 어시스턴트</span>
      </div>

      <div className="ai-actions">
        {actions.map((action) => (
          <button
            key={action.id}
            onClick={() => handleAction(action.id)}
            className="ai-action-btn"
          >
            {action.icon}
            <span>{action.label}</span>
          </button>
        ))}
      </div>

      {isLoading && <LoadingDots />}

      {suggestions.length > 0 && (
        <div className="ai-suggestions">
          {suggestions.map((suggestion, i) => (
            <SuggestionCard
              key={i}
              suggestion={suggestion}
              onApply={() => applySuggestion(suggestion)}
            />
          ))}
        </div>
      )}
    </div>
  );
};
```

#### 3.5.2 인라인 AI 제안

```jsx
// components/editor/InlineSuggestion.jsx
export const InlineSuggestion = ({ suggestion, position, onAccept, onReject }) => (
  <div
    className="inline-suggestion"
    style={{ top: position.top, left: position.left }}
  >
    <div className="suggestion-content">
      <span className="suggestion-text">{suggestion.text}</span>
      <span className="suggestion-diff">{suggestion.diff}</span>
    </div>
    <div className="suggestion-actions">
      <button onClick={onAccept} className="accept-btn">
        <CheckIcon /> 적용
      </button>
      <button onClick={onReject} className="reject-btn">
        <XIcon /> 취소
      </button>
      <span className="shortcut-hint">Tab으로 적용</span>
    </div>
  </div>
);
```

### Phase 6: Accessibility & Performance (2주)

#### 3.6.1 접근성 개선

```jsx
// components/ui/AccessibleButton.jsx
export const AccessibleButton = ({
  children,
  onClick,
  ariaLabel,
  disabled,
  loading,
  ...props
}) => (
  <button
    onClick={onClick}
    disabled={disabled || loading}
    aria-label={ariaLabel}
    aria-busy={loading}
    aria-disabled={disabled}
    {...props}
  >
    {loading ? <LoadingSpinner /> : children}
  </button>
);

// hooks/useFocusTrap.js
export const useFocusTrap = (ref, isActive) => {
  useEffect(() => {
    if (!isActive || !ref.current) return;

    const focusableElements = ref.current.querySelectorAll(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );

    const firstElement = focusableElements[0];
    const lastElement = focusableElements[focusableElements.length - 1];

    const handleKeyDown = (e) => {
      if (e.key !== 'Tab') return;

      if (e.shiftKey && document.activeElement === firstElement) {
        e.preventDefault();
        lastElement.focus();
      } else if (!e.shiftKey && document.activeElement === lastElement) {
        e.preventDefault();
        firstElement.focus();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    firstElement?.focus();

    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [ref, isActive]);
};
```

#### 3.6.2 성능 최적화

```jsx
// Virtualized List for large document lists
import { FixedSizeList as List } from 'react-window';

export const VirtualDocumentList = ({ documents }) => (
  <List
    height={600}
    itemCount={documents.length}
    itemSize={80}
    width="100%"
  >
    {({ index, style }) => (
      <div style={style}>
        <DocumentListItem document={documents[index]} />
      </div>
    )}
  </List>
);

// Image lazy loading
export const LazyImage = ({ src, alt, ...props }) => (
  <img
    src={src}
    alt={alt}
    loading="lazy"
    decoding="async"
    {...props}
  />
);

// Code splitting for routes
const routes = [
  { path: '/', element: lazy(() => import('./pages/Home')) },
  { path: '/documents/:id', element: lazy(() => import('./pages/DocumentView')) },
  { path: '/admin/*', element: lazy(() => import('./pages/Admin')) },
];
```

---

## 4. Design System

### 4.1 Color Palette

```css
:root {
  /* Primary */
  --color-primary-50: #eff6ff;
  --color-primary-100: #dbeafe;
  --color-primary-500: #3b82f6;
  --color-primary-600: #2563eb;
  --color-primary-700: #1d4ed8;

  /* Neutral */
  --color-gray-50: #f9fafb;
  --color-gray-100: #f3f4f6;
  --color-gray-200: #e5e7eb;
  --color-gray-300: #d1d5db;
  --color-gray-500: #6b7280;
  --color-gray-700: #374151;
  --color-gray-900: #111827;

  /* Semantic */
  --color-success: #10b981;
  --color-warning: #f59e0b;
  --color-error: #ef4444;
  --color-info: #3b82f6;

  /* Dark Mode */
  --color-dark-bg: #1f2937;
  --color-dark-surface: #374151;
  --color-dark-text: #f9fafb;
}
```

### 4.2 Typography

```css
:root {
  /* Font Family */
  --font-sans: 'Pretendard', -apple-system, BlinkMacSystemFont, system-ui, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;

  /* Font Size */
  --text-xs: 0.75rem;    /* 12px */
  --text-sm: 0.875rem;   /* 14px */
  --text-base: 1rem;     /* 16px */
  --text-lg: 1.125rem;   /* 18px */
  --text-xl: 1.25rem;    /* 20px */
  --text-2xl: 1.5rem;    /* 24px */
  --text-3xl: 1.875rem;  /* 30px */

  /* Line Height */
  --leading-tight: 1.25;
  --leading-normal: 1.5;
  --leading-relaxed: 1.75;
}
```

### 4.3 Spacing

```css
:root {
  --space-1: 0.25rem;  /* 4px */
  --space-2: 0.5rem;   /* 8px */
  --space-3: 0.75rem;  /* 12px */
  --space-4: 1rem;     /* 16px */
  --space-6: 1.5rem;   /* 24px */
  --space-8: 2rem;     /* 32px */
  --space-12: 3rem;    /* 48px */
  --space-16: 4rem;    /* 64px */
}
```

### 4.4 Component Library

```jsx
// components/ui/index.js
export { Button } from './Button';
export { Input } from './Input';
export { Select } from './Select';
export { Modal } from './Modal';
export { Toast } from './Toast';
export { Tooltip } from './Tooltip';
export { Dropdown } from './Dropdown';
export { Tabs } from './Tabs';
export { Badge } from './Badge';
export { Avatar } from './Avatar';
export { Card } from './Card';
export { Skeleton } from './Skeleton';
export { Spinner } from './Spinner';
```

---

## 5. Implementation Timeline

```
Week 1-2: Phase 1 (Quick Wins)
├── 지연 로딩 구현
├── 스켈레톤 UI 추가
├── Toast 알림 시스템
└── 기본 성능 최적화

Week 3-5: Phase 2 (Mobile & Responsive)
├── 반응형 레이아웃
├── 모바일 네비게이션
├── 터치 제스처
└── PWA 설정

Week 6-7: Phase 3 (Search & Discovery)
├── 고급 검색 UI
├── 실시간 자동완성
└── 필터 시스템

Week 8-10: Phase 4 (Editor Experience)
├── 협업 시각화
├── 자동 저장 UI
├── 키보드 단축키
└── 마크다운 툴바

Week 11-12: Phase 5 (AI Integration)
├── AI 어시스턴트 패널
├── 인라인 제안
└── 태그 추천 UI

Week 13-14: Phase 6 (Polish)
├── 접근성 개선
├── 성능 최적화
├── 다크 모드
└── 문서화
```

---

## 6. Success Metrics

| 메트릭 | 현재 | 목표 |
|--------|------|------|
| First Contentful Paint | ~2.5s | <1s |
| Time to Interactive | ~4s | <2s |
| Lighthouse Score | 65 | 90+ |
| Mobile Usability | 70% | 95%+ |
| Accessibility Score | 60 | 90+ |
| 사용자 만족도 (NPS) | - | 8+ |

---

## 7. Tech Stack Upgrades

### 현재 → 목표

| 영역 | 현재 | 목표 |
|------|------|------|
| Styling | CSS Modules | **Tailwind CSS** |
| State | Context + useState | **Zustand** |
| Forms | Manual | **React Hook Form** |
| Animation | CSS | **Framer Motion** |
| Testing | - | **Vitest + Playwright** |
| Build | CRA | **Vite** |

### Tailwind 마이그레이션 예시

```jsx
// Before (CSS Modules)
import styles from './Button.module.css';

<button className={styles.primaryButton}>Click</button>

// After (Tailwind)
<button className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
  Click
</button>
```

---

## 8. Conclusion

이 UI/UX 개선 계획은 14주에 걸쳐 단계적으로 구현되며, 각 단계는 독립적으로 배포 가능합니다. 모바일 반응형, 검색 UX, 편집 경험, AI 통합에 중점을 두어 사용자 경험을 크게 향상시킬 수 있습니다.

Rust 백엔드 마이그레이션과 병행하여 진행하면, 완전히 현대화된 MinKy 2.0을 완성할 수 있습니다.
