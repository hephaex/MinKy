# minky Frontend UI/UX Review

**Review date:** 2026-06-21  
**Files reviewed:**

- `frontend/src/components/TreeView.js`
- `frontend/src/components/TreeView.css`
- `frontend/src/components/TreeView.test.js`
- `frontend/src/components/TreeSidebar.js`
- `frontend/src/components/TreeSidebar.css`
- `frontend/src/components/SearchBar.js`
- `frontend/src/components/SearchBar.css`
- `frontend/src/components/Search/SearchBar.jsx`
- `frontend/src/components/Search/SearchBar.css`

---

## 1. Accessibility (ARIA tree role, keyboard, focus, screen reader)

### Current state

- The tree root has `role="tree"` with `tabIndex={0}` and a global `onKeyDown` handler for `ArrowDown`/`ArrowUp`.
- Each node has `role="treeitem"`, `tabIndex={isFocused ? 0 : -1}`, `aria-expanded` (folders only), and `aria-selected` (documents only).
- Focus is managed via a `focusedId` state + `useEffect` that calls `.focus()` on the DOM node when `isFocused` changes.
- ArrowRight/ArrowLeft on a node toggle expand/collapse (with `preventDefault`). Enter/Space both call `handleClick()`.
- The `handleTreeKeyDown` ArrowDown/ArrowUp walks the *visible* flattened list of IDs and sets `focusedId`.

### Problems

| # | Severity | Issue | Detail |
|---|----------|-------|--------|
| A1 | HIGH | **Roving tabindex pattern is partially broken** | Only one item has `tabIndex={0}` (the focused one). This is the correct shape for a roving tabindex *roving* pattern, but the global `ArrowDown/ArrowUp` handler only moves `focusedId` — it **does not move `tabIndex`/`focus` around the treeitem elements**. The `useEffect` that calls `.focus()` on the *new* focusedId runs *after* render, creating an async focus jump that can flicker and can lose focus if the newly-focused node is not yet rendered (collapsed ancestor). Test `it('handles keyboard navigation with ArrowDown')` only verifies `tabIndex` attribute value after `fireEvent.keyDown`, not that focus actually moved. |
| A2 | HIGH | **Enter and Space duplicate behavior for folders** | `handleKeyDown` calls `handleClick()` on both `Enter` and ` ` (Space). `handleClick()` on a folder calls `onToggle`. `handleArrowClick` (the chevron button) *also* calls `onToggle` with `e.stopPropagation()`. WAI-ARIA tree pattern expects **Enter = activate** (select/navigate) and **Space/ArrowRight = toggle expand/collapse** — not both keys doing the same thing. In practice this means pressing Space on a folder also toggles (coincidental alignment), but then clicking the chevron *also* toggles — this is the double-expansion bug surface. |
| A3 | MEDIUM | **`aria-level` is missing** | WAI-ARIA treeitem requires `aria-level` to indicate depth. Without it, screen readers cannot convey nesting depth. Current code only sets `aria-expanded` and `aria-selected`. |
| A4 | MEDIUM | **`aria-label` on expand chevron is Korean-only (`접기`/`펼치기`)** | If the app is localized, this hardcoded Korean label will mislead Korean-incompetent screen reader users. Should use localized strings or `aria-hidden="true"` on the icon + `aria-label` on the parent treeitem. |
| A5 | MEDIUM | **`aria-selected` only set on documents, never folders** | WAI-ARIA tree pattern: `aria-selected` should be set on the *focused* item regardless of type. Currently a focused folder does not get `aria-selected`. The comment in TS documentation often says "a treeitem can be both expanded and selected" — this is not supported. |
| A6 | MEDIUM | **No `aria-label`/`aria-labelledby` on the tree root** | Screen readers announce the tree by its accessible name. The current `role="tree"` has no accessible name. Should have `aria-label="Documents tree"` or `aria-labelledby` pointing to the sidebar heading. |
| A7 | LOW | **Focus wrap not explicit** | `ArrowDown` wraps from last visible to first; `ArrowUp` wraps from first to last. WAI-ARIA does not mandate wrap, but if the design intent is wrap, it should be tested and documented. If the intent is *no wrap*, current code is wrong. |
| A8 | LOW | **Collapse can orphan focus** | If focused node is inside a collapsed subtree, collapsing the parent removes the focused node from the DOM. The `useEffect` tries to call `.focus()` on a node that no longer exists — no error thrown (React guards), but focus is lost silently. This is a WCAG 2.1 Success Criterion 2.4.3 violation (Focus Order). |

### Recommendations

- Adopt a **single focused item** model where `tabIndex={0}` always stays on the focused item, and `onKeyDown` moves focus *and* `focusedId` together synchronously (imperative `ref.focus()` in the key handler, not via `useEffect`).
- Route `Enter` to `onSelect` (activate), route `Space` and `ArrowRight` to `onToggle` (expand/collapse) — send `e.preventDefault()` in each. Remove `handleClick` call from Space.
- Add `aria-level={level + 1}` to every treeitem.
- Localize chevron `aria-label`, or remove `aria-label` and add `aria-hidden="true"` to the span + `role="button"` only if it gets `tabIndex` — current pattern has `tabIndex={-1}` implicitly on the icon, so `aria-hidden="true"` is safer.
- Set `aria-selected={focusedId === node.id}` on every treeitem, not just documents.
- Add `aria-label="Documents tree"` to the tree container.
- Test focus preservation when collapsing a parent of the focused node: if focusedId is in a collapsed subtree, programmatically move focus to the collapsed parent node.

---

## 2. TreeView correctness — double expansion on folder row click

### Current behavior

```
TreeNode handleClick():
  if (isDocument) → onSelect(node)
  else if (hasChildren) → onToggle(node.id)   // folder row click TOGGLES
  else if (onNodeClick) → onNodeClick(node)

TreeNode handleArrowClick():
  e.stopPropagation()
  if (hasChildren) → onToggle(node.id)        // chevron click TOGGLES
```

Result: clicking anywhere on a folder row toggles expansion **AND** the chevron button also toggles expansion with `stopPropagation`. For a user who clicks the row *next to* the chevron, this is intuitive. For a user who clicks *on* the chevron, `stopPropagation` prevents the row handler from firing — so no double-toggle on a single click today.

### The real bug: interaction design, not just code

The current design does **not** cause double expansion from a single click because `e.stopPropagation()` on the chevron prevents the row handler. However, the design is **incorrect for a standard tree widget**:

| Action | Expected (WAI-ARIA tree pattern) | Current |
|--------|-----------------------------------|---------|
| Click chevron | Toggle expand/collapse | Toggle expand/collapse ✓ |
| Click folder label/name | Activate/select (navigate to folder view, or select folder) | Toggle expand/collapse ✗ |
| Double-click folder label | Toggle expand/collapse (already indented behavior) | Double-toggle on double-click |

### What should happen

**Recommended model (matches macOS Finder / VS Code / WAI-ARIA best practice):**

- **Chevron click** → toggle expand/collapse (current — keep).
- **Folder row click** (on label, not chevron) → **select/focus the folder**, do NOT toggle expansion. This is the activation model. In the sidebar use case, selecting a folder navigates to `/tags/:slug` or `/explore-date?date=` via `handleNodeClick` in TreeSidebar.
- **Enter on folder** → activate/select (navigate), NOT toggle. This matches the "activation" model.
- **Space / ArrowRight on folder** → toggle expand/collapse (keep as shortcut for power users).
- **Double-click on folder label** → toggle expand/collapse (convenience shortcut).

Current code conflates "activate" and "toggle" into the same handler. This means there is no way for a parent consumer to distinguish "user clicked the folder label to select it" from "user clicked the folder label to expand it." In TreeSidebar, `handleNodeClick` (navigation) is wired to the `onNodeClick` prop, but `onNodeClick` is only reached for folders **without children** (leaf folders / "custom navigation" fallback). Folders *with* children *always* toggle and never reach `onNodeClick`.

**Concrete code change needed:**

```javascript
// Replace current handleClick with two-mode behavior:
const handleClick = (e) => {
  // If the click target is the chevron icon, ignore (already handled by handleArrowClick)
  if (e.target.closest('.tree-expand-icon')) return;

  if (isDocument) {
    onSelect(node);
  } else if (hasChildren) {
    // Folder with children: select it (do not toggle)
    onNodeClick?.(node);
    // Also set focus / selection state if needed
  } else {
    // Leaf folder without children — use onNodeClick for navigation
    onNodeClick?.(node);
  }
};
```

And keep `handleArrowClick` as the **only** mouse-triggered expansion toggle (plus `Space`/`ArrowRight` keyboard shortcuts).

### Migration note

This changes the behavior of existing consumers who rely on row-click-to-toggle (if any). At time of review, the only known consumer is `TreeSidebar.js`, which already has `handleNodeClick` wired but has never received it for folders-with-children. The migration is therefore: **no behavior regression for existing code** because `onNodeClick` was unreachable for expandable folders before. After the change, `onNodeClick` fires on folder label click and the consumer navigates.

---

## 3. Visual design consistency — CSS class naming, spacing, responsive, dark mode

### CSS class naming

TreeView uses a `tree-*` prefix (e.g., `.tree-view`, `.tree-node`, `.tree-node-label`, `.tree-children`). This is **consistent and good** — follows a namespaced BEM-ish pattern. No conflicts with other component CSS observed. No camelCase or kebab-case mixing; all lowercase with hyphens only ✓

### Spacing issues

| Issue | Detail |
|-------|--------|
| **Hardcoded `gap: 1px` on `.tree-view`** | This 1px gap creates a hairline border between nodes. Makes hover background harder to interpret (the 1px gap is the background color between two hovered nodes). Consider using margin on `.tree-node` instead, or border on nodes. |
| **Indentation is inline style only** | `style={{ paddingLeft: \`${level * 16 + 8}px \`}}` — magic number `16` (indent per level) and `8` (base padding) are in JS, not CSS. Not easily themeable or adjustable via CSS custom properties. The `8px` base offset does not match the 8px grid used elsewhere (e.g., `.sidebar-section` padding, `.tree-node-label` padding). |
| **`.tree-node-leaf:hover` has `border-left-color: #ff6b35`** | This orange border on hover for leaf nodes is the **only** color accent in the tree (other than the expand chevron). It is not reflected in the focus ring (which is `#555`). Creates an inconsistent "selected item" treatment: hover shows orange left border, focus shows grey outline-1. Should pick one selected/hover indicator. |
| **Branch vs leaf hover colors too similar** | `.tree-node-branch:hover` has `background-color: #f0f4ff; color: #1a56db` (light blue background + dark blue text). `.tree-node:hover` (default, used for leaf nodes) has `background-color: #2a2a2a` (dark grey). The parent `.tree-node` hover applies to both leaf and branch — but `.tree-node-branch:hover` **overrides** it. This means folder nodes on hover turn blue, leaf nodes turn dark grey. Inconsistent affordance — folder "hover" looks like a selected state while leaf "hover" looks like a highlight. |

### Responsive design

`TreeView.css` has **no media queries at all**. The tree is not responsive. The parent sidebar (`DocumentsSidebar.css`) has a `@media (max-width: 768px)` rule that hides the sidebar with `transform: translateX(-100%)`. But the tree itself does not react to:

- Narrow widths (e.g., on tablets, the tree should switch to a single-column layout with disclosure indicators)
- Very long node labels (current `text-overflow: ellipsis` + `white-space: nowrap` on `.tree-node` and `.tree-node-label` handles it, but the `.tree-node-color` dot and `.tree-node-count` badge are always visible and can push the label truncation before the indicator; the indicator SVG sits *before* the label via flex order, so for very long labels on small screens, the count badge can be clipped in an ugly way — currently no CSS clamping on the label width)

### Dark mode

All colors are hardcoded hex values. No CSS custom properties (`var(--...)`) and no `@media (prefers-color-scheme: dark)` queries. The app appears to use a dark theme as default (the sidebar is dark `#e5e5e5` text on dark backgrounds), but there is no explicit dark/light mode architecture, so theming is not extensible.

### Color contrast

- `.tree-node` text is `#e5e5e5` on default background (assumed `#1a1a1a` or similar from sidebar). This passes WCAG AA for normal text (≥4.5:1 ratio) at large text sizes only — at small text (`13px` font-size), the 4.5 ratio requirement means the background must be at most `#6e6e6e` on `#e5e5e5` text. If the background is `#1a1a1a`, ratio is ~14:1 — passes ✓. On hover (`#2a2a2a`), ratio is ~12:1 — passes ✓.
- `.tree-expand-icon` color is `#888` — on the light blue hover background (`#f0f4ff`), this fails contrast (grey on white). But the icon has `fill="currentColor"` so it inherits the tree-node color (`#1a56db` on branch hover). The hover state changes icon color with the node, which is good.

---

## 4. UX for tree interactions — empty tree, deep nesting, label truncation

### Empty tree state

Current implementation: `.tree-empty` class renders `<div className="tree-empty">문서가 없습니다</div>` ("No documents").

- **Issues:**
  - The message is Korean-only, but `TreeSidebar` is international-facing (sidebar title is "Documents", tabs are Korean). Mixed language.
  - The empty message is rendered inside the `role="tree"` container, not as a sibling. ARIA best practice: empty tree should use `aria-empty="true"` on the tree container, or render a separate announcement element with `aria-live`.
  - No icon or visual cue. A simple "No documents" text line in a grey italic box is functional but not inviting.
- **Recommendation:** Add `aria-empty="true"` when `nodes.length === 0`. Localize the message. Consider adding an icon or illustration.

### Deep nesting

Current handling: recursive `<TreeNode>` renders with `level` parameter. Indentation grows linearly (`level * 16px`). No virtual scrolling.

- **Issues:**
  - No maximum level clamp: a tree with 50 levels (pathological) would indent 800px, causing horizontal scroll that is not scrollable (no `overflow-x: auto` on the tree container).
  - No visual "collapse to reference" behavior for very deep paths. In most professional trees (file explorers, etc.), after ~5 levels the tree either stops indenting, collapses mid-path to a reference (e.g., `... > subfolder`), or virtualizes.
  - `getAllVisibleIds` is recomputed on every keydown. For very large trees with many expanded nodes, this recursive traversal is O(N) on every ArrowUp/ArrowDown. Could be memoized.
- **Recommendation:** Clamp indentation at a maximum (e.g., `Math.min(level, 8) * 16 + 8`). Add `overflow-x: auto` on `.tree-view` as a safety. Consider a maximum rendered depth or "flatten" display after depth > 5.

### Label truncation

- CSS: `.tree-node-label` has `flex: 1`, `overflow: hidden`, `text-overflow: ellipsis`, `white-space: nowrap`. This handles truncation well.
- **Issues:**
  - The truncate is on the label element only. The `title={node.label}` attribute provides a native tooltip, which is fine for mouse users, but **not accessible to touch/screen-reader users**.
  - `.tree-node` itself also has `overflow: hidden` and `text-overflow: ellipsis` — this means if the tree-node container is too narrow (narrow sidebar), the label truncation + count badge could all be clipped in an odd way because the count badge is the last flex item and the spacer is before it.
  - The `📄` emoji icon is prepended to every document name. This is visually appealing but not screen-reader friendly — it is announced as a Unicode character by most screen readers. Should either be hidden from AT with `aria-hidden="true"` or replaced with a CSS pseudo-element.

### Recommendations

- Wrap emoji in `<span aria-hidden="true">📄</span>` and add a visually-hidden screen reader label if the emoji conveys information (category: document).
- Ensure `title` tooltips have equal information available to screen readers via `aria-label` or visually-hidden full-name span.
- Add `aria-live="polite"` region for empty tree announcements: `<div role="status">문서가 없습니다</div>`.

---

## 5. State management clarity

### TreeView component

- **State:**
  - `expandedNodes: Set<id>` — which folders are open.
  - `focusedId: string | null` — which item has focus (via roving tabindex).
- **Props:** `nodes`, `onSelect`, `onNodeClick`, `className`.

The code uses a **single `focusedId`** for both "which item has roving tabindex" and "which item is focused in the DOM". This works but conflates **focus for keyboard navigation** with **selection for persistence across re-renders**. The `useEffect` syncs the DOM focus to `focusedId`, which is a reasonable indirection, but means:

1. If a parent re-renders `TreeView` with new nodes (same IDs but different shape), the `useEffect` will focus the node with matching ID — but if the node content changed focusable semantics (e.g., a newly expandable folder was previously a leaf), the focus call may fail silently.
2. `focusedId` can reference a node ID that is not currently visible (collapsed parent). The `getAllVisibleIds` function filters to visible IDs, but the `focusedId` state still holds the invisible ID, leading to `ids.indexOf(focusedId) === -1` on every ArrowDown/ArrowUp until the focused parent is expanded again. This means focus navigation silently resets to index 0 after expanding a parent of the previously-focused node — unexpected but not tested.

### TreeSidebar component

- **State:** `mode`, `treeData`, `loading`, `error`, `searchQuery`.
- **Derived state:** `filteredTree` is a `useMemo` that filters `treeData` by `searchQuery`.
- **Issue:** The `useMemo` for `filterNode` creates new objects with `{ ...node, children: filteredChildren, count: computedCount }`. The `count` field is **mutated** in the filter: it is overwritten with the *filtered* child document count, not the original tree count. This means when searching, folder counts show only matching documents. This is arguably correct UX (only show relevant counts), but it is worth documenting because it is a semantic change to the tree data during render. If the consumer (e.g., a legend that says "X documents in folder") was relying on the original count, it would break.

### Consistency

- `TreeView` uses `Set` for `expandedNodes`; `TreeSidebar` does not expose expand state to the consumer — it is self-contained. If the consumer wanted to synchronize multiple trees or restore expand state, there is no API for this.
- No loading or error state props on `TreeView` itself; the parent (`TreeSidebar`) shows a `.loading-indicator` child outside of `TreeView`. This is correct separation of concerns, but the `TreeView` empty state is "No documents" which overlaps with the loading state — a hasty loading→empty transition could flash the empty message.

### Recommendations

- Add `initialExpandedIds?: Set<id>` prop to `TreeView` for controlled expand state if needed.
- Document the `filterNode` count mutation behavior.
- Add loading/error state props to `TreeView` for flexibility: `TreeView` could render its own inline loading skeleton rather than relying on the parent to inject a sibling spinner.

---

## 6. Loading & error states

### Current implementation

In `TreeSidebar.js`:

```jsx
<div className="sidebar-section tree-section">
  {loading ? (
    <div className="loading-indicator">로딩 중...</div>
  ) : error ? (
    <div className="empty-state">{error}</div>
  ) : (
    <TreeView nodes={filteredTree} onSelect={handleSelect} onNodeClick={handleNodeClick} />
  )}
</div>
```

- **Loading:** Renders `"로딩 중..."` ("Loading...") as a plain text div. No spinner, no animation. The CSS for `.loading-indicator` is in `DocumentsSidebar.css` but is not reviewed in detail — assuming it is a simple text block based on the `text-align: center` style.
- **Error:** Renders the error message inside `.empty-state`, reusing the empty state styling. No "retry" action. After a network error, the user sees an error message but no way to retry without changing tabs or refreshing.

### TreeView empty/error state overlap

- `TreeView` has its own internal empty state: `<div className="tree-empty">문서가 없습니다</div>`.
- When `TreeSidebar` has an error, it hides the `TreeView` entirely and shows the error div in its place. When `loading` is true, same pattern.
- But when `treeData` is `[]` (truly empty — user has no documents), `TreeSidebar` passes `filteredTree = []` to `TreeView`, and `TreeView` renders its empty message. This works, but two different conditions (error vs empty) both result in a text message — one inside `TreeSidebar`, one inside `TreeView`. The visual consistency depends on the CSS of `.empty-state` (in sidebar) vs `.tree-empty` (in TreeView). A quick look:

| Class | Background | Text color | Font style |
|-------|-----------|------------|------------|
| `.tree-empty` | transparent (default) | `#666` | italic |
| `.empty-state` (in DocumentsSidebar.css) | ? | ? | ? |

Not fully reviewed, but likely inconsistent given different authors.

### Recommendations

- Add a retry button inside the error state in `TreeSidebar`:
  ```jsx
  <div className="empty-state">
    <span>{error}</span>
    <button onClick={() => fetchTree(mode)}>다시 시도</button>
  </div>
  ```
- Distinguish "empty" from "error" visually if the current CSS does not. Consider a red-tinted error state vs a neutral empty state.
- TreeView `tree-empty` should strip `italic` and give it a subtle background (e.g., `#1e1e1e`) to distinguish it from a content-less tree.
- Loading state: use `<LoadingSpinner>` component if available, or at minimum add a CSS spinner animation in `.loading-indicator`.

---

## 7. Unused CSS — TreeView.css class coverage

### All classes in TreeView.css

| Selector | Used in TreeView.js? | Notes |
|----------|----------------------|-------|
| `.tree-view` | Yes: `<div className="tree-view ...">` | ✓ |
| `.tree-node` | Yes: `<div className={`tree-node ${...} ${...}`}>` | ✓ |
| `.tree-node:hover` | — (pseudo-class) | ✓ |
| `.tree-node-focused` | Yes: conditionally added | ✓ |
| `.tree-node-leaf` | Yes: conditionally added | ✓ |
| `.tree-node-leaf:hover` | — (pseudo-class) | ✓ |
| `.tree-node-branch` | Yes: conditionally added | ✓ |
| `.tree-node-branch:hover` | — (pseudo-class) | ✓ |
| `.tree-expand-icon` | Yes: `<span className={`tree-expand-icon ${...}`}>` | ✓ |
| `.tree-expand-icon:hover` | — (pseudo-class) | ✓ |
| `.tree-expand-icon.expanded` | Yes: conditionally added | ✓ |
| `.tree-expand-spacer` | Yes: `{!hasChildren && <span className="tree-expand-spacer" />}` | ✓ |
| `.tree-node-color` | Yes: `{node.color && <span className="tree-node-color" ...>}` | ✓ |
| `.tree-node-label` | Yes: `<span className="tree-node-label">` | ✓ |
| `.tree-node-count` | Yes: `{node.count !== undefined && <span className="tree-node-count">}` | ✓ |
| `.tree-children` | Yes: `<div className="tree-children" role="group">` | ✓ |
| `.tree-empty` | Yes: `{!nodes \|\| nodes.length === 0 ? <div className="tree-empty">...` | ✓ |

### All classes in TreeView.js — covered in CSS?

| Class in JS | Defined in TreeView.css? | Notes |
|------------|--------------------------|-------|
| `tree-node-container` | **NO** | Used on the outer wrapper `<div>` around each node: `<div className="tree-node-container">`. The CSS file has `.tree-node` but **not** `.tree-node-container`. The container is a layout wrapper that currently carries no styles, so it is a CSS gap. Could be removed (flatten to direct child) or have styles added. |
| `tree-node` | ✓ | |
| `tree-node-leaf` | ✓ | |
| `tree-node-branch` | ✓ | |
| `tree-node-focused` | ✓ | |
| `tree-expand-icon` | ✓ | |
| `expanded` (on `.tree-expand-icon`) | ✓ (`.tree-expand-icon.expanded`) | |
| `tree-expand-spacer` | ✓ | |
| `tree-node-color` | ✓ | |
| `tree-node-label` | ✓ | |
| `tree-node-count` | ✓ | |
| `tree-children` | ✓ | |
| `tree-empty` | ✓ (in TreeView.css) | |

**Unused CSS class: `.tree-node-container` — defined in JS but has no CSS rules.** It acts as a transparent wrapper. The `tree-children` div inside also has `display: flex; flex-direction: column;` which applies to the children container, not the node container — this is fine, but the wrapper is not styled, suggesting it was either a leftover from a refactoring or a placeholder for future styles.

### Recommendations

- Remove `.tree-node-container` wrapper unless it will need styles (e.g., for drag-and-drop visual feedback, or animation). Flatten it: render `<TreeNode>` content directly in the parent `nodes.map()`.
- Alternatively, add a comment `// TODO: layout wrapper for drag-and-drop` to clarify intent.

---

## Summary of actionable items, ranked by priority

| Priority | Area | Action |
|----------|------|--------|
| 🔴 P0 | TreeView correctness | Separate folder click (select/activate) from expand toggle — row click navigates, chevron/Space/ArrowRight toggles. This fixes the double-expansion conceptual bug. |
| 🔴 P0 | Accessibility | Add `aria-level`, fix roving tabindex sync (focus moves immediately in key handler, not via useEffect), set `aria-selected` on all items, localize aria-labels. |
| 🟠 P1 | Accessibility | Add focus preservation logic: when collapsing a parent of focused node, move focus to the collapsed parent. |
| 🟠 P1 | Loading/error | Add retry button to error state in TreeSidebar. |
| 🟡 P2 | UX | Remove `.tree-node-container` wrapper (unused CSS). |
| 🟡 P2 | UX | Clamp indentation at max level and add `overflow-x: auto` on tree container. |
| 🟡 P2 | UX | Make emoji icon `aria-hidden="true"` on document labels. |
| 🟢 P3 | Visual design | Fix hover state inconsistency between branch/leaf nodes. |
| 🟢 P3 | Visual design | Convert hardcoded colors to CSS custom properties for theming. |
| 🟢 P3 | Visual design | Localize the "문서가 없습니다" empty state message. |
