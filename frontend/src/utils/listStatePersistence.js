// Persist a list view's state (page + filters) in sessionStorage so that
// navigating into a detail page and back restores the previous view instead
// of resetting to page 1. Used by the document/tag/admin list views.

export const loadListState = (key) => {
  try {
    const raw = sessionStorage.getItem(key);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
};

export const saveListState = (key, state) => {
  try {
    sessionStorage.setItem(key, JSON.stringify(state));
  } catch {
    // sessionStorage unavailable or quota exceeded — non-fatal
  }
};
