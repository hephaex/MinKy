// 문서 내 목차(TOC) 유틸리티 — DocumentView가 렌더링하는 heading에 부여하는 id와
// DocumentToc가 표시하는 목차 항목의 앵커가 항상 일치하도록 두 곳에서 공유해서 쓴다.

/**
 * Slugify heading text into a URL/DOM-safe anchor id. Keeps Unicode letters
 * (so Korean headings get readable ids), strips markdown emphasis/code
 * markers, collapses whitespace/punctuation runs into single hyphens.
 */
export function slugify(text) {
  const slug = String(text)
    .trim()
    .toLowerCase()
    .replace(/[`*_~]/g, '') // strip inline emphasis/code markers
    .replace(/[^\p{L}\p{N}\s-]/gu, '') // strip punctuation, keep unicode letters/numbers
    .trim()
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-');
  return slug || 'section';
}

/**
 * Extract ATX-style (`#` .. `######`) headings from raw markdown, skipping
 * anything inside fenced code blocks (``` or ~~~) so a `# comment` in a code
 * sample is never mistaken for a heading. Returns one entry per heading with
 * a de-duplicated slug (GitHub-style `-1`, `-2` suffixes) so two headings
 * with the same text still get distinct anchors.
 *
 * Must run on the exact same string DocumentView passes to `ReactMarkdown`
 * (post `processInternalLinks`/`processHashtags`) — DocumentView assigns
 * heading ids by *position* (the Nth heading encountered during render), not
 * by re-deriving the slug, so this function's heading order/count is the
 * single source of truth both sides rely on.
 */
export function extractHeadings(markdown) {
  if (!markdown) return [];
  const lines = markdown.split('\n');
  const headings = [];
  const seen = new Map();
  let inFence = false;
  let fenceMarker = null;

  for (const line of lines) {
    const fenceMatch = line.match(/^\s*(```|~~~)/);
    if (fenceMatch) {
      if (!inFence) {
        inFence = true;
        fenceMarker = fenceMatch[1];
      } else if (fenceMatch[1] === fenceMarker) {
        inFence = false;
        fenceMarker = null;
      }
      continue;
    }
    if (inFence) continue;

    const headingMatch = line.match(/^(#{1,6})\s+(.+?)\s*#*\s*$/);
    if (!headingMatch) continue;

    const level = headingMatch[1].length;
    const text = headingMatch[2].trim();
    if (!text) continue;

    let slug = slugify(text);
    const count = seen.get(slug) || 0;
    seen.set(slug, count + 1);
    if (count > 0) slug = `${slug}-${count}`;

    headings.push({ level, text, slug });
  }

  return headings;
}
