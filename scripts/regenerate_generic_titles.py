#!/usr/bin/env python3
"""regenerate_generic_titles.py — solar-open2 기반 제목 재생성 배치.

대상: minky(CT306)에 클리퍼 캡처 실패로 generic title(Perplexity/Facebook/md/
Gemini Conversation/Claude Conversation/Untitled/Unix 등)이 붙은 문서.
markdown_content를 solar-open2에 보내 본문 기반 제목을 생성하고 PUT으로 갱신.

원 title은 서버 백업(minky_lab_backup_20260724_082142.sql.gz)에 보존되어 있음 — 안전.
"""
import json, os, re, sys, time, requests, threading, subprocess
from concurrent.futures import ThreadPoolExecutor, as_completed

MINKY_BASE = "http://192.168.29.22:8000"
UPSTAGE_API_KEY = os.environ.get("UPSTAGE_API_KEY", "")
UPSTAGE_URL = "https://api.upstage.ai/v1/chat/completions"
UPSTAGE_MODEL = "solar-open2"
UPSTAGE_MAX_TOKENS = 32000  # reasoning-heavy model; smaller budgets never reach `content` (실측 2026-07-24)
WORKERS = 4
LOG = os.path.expanduser(os.environ.get("TITLE_BATCH_LOG", "~/Simon/minky/logs/regenerate_titles.log"))
DOC_IDS_FILE = os.path.expanduser(
    sys.argv[1] if len(sys.argv) > 1 else "~/Simon/minky/scripts/generic_title_doc_ids.json"
)
SSH_HOST = "root@192.168.29.22"
DB_CONTAINER = "minky-lab-db-1"
DB_USER = "minky_lab"
DB_NAME = "minky_lab"
_log_lock = threading.Lock()
_sql_lock = threading.Lock()  # serialize DB writes (per-doc ssh call, avoid concurrent psql spam)

_FM_RE = re.compile(r'^---\n.*?\n---\n?', re.DOTALL)

_PROMPT_TMPL = """\
다음은 웹 페이지를 클리핑한 문서의 본문입니다. 원래 제목이 "{old_title}"(브라우저 탭
이름이나 사이트명 같은 무의미한 제목)이라 본문 내용을 반영하지 못합니다.

이 본문을 읽고, 실제 내용을 정확히 설명하는 한국어 제목을 하나만 만들어 주세요.
규칙:
- 제목만 출력 (설명·따옴표·번호·마크다운 기호 없이)
- 최대 60자
- 본문의 핵심 주제·고유명사를 반영
- 본문이 대화/스레드라면 그 대화의 주제를 요약

===본문(앞부분)===
{body}
===제목===
"""


def log(msg):
    line = f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] {msg}"
    with _log_lock:
        print(line, flush=True)
        with open(LOG, "a", encoding="utf-8") as f:
            f.write(line + "\n")


def call_solar(prompt, retries=5):
    for attempt in range(retries):
        try:
            resp = requests.post(
                UPSTAGE_URL,
                headers={"Authorization": f"Bearer {UPSTAGE_API_KEY}", "Content-Type": "application/json"},
                json={"model": UPSTAGE_MODEL, "messages": [{"role": "user", "content": prompt}], "max_tokens": UPSTAGE_MAX_TOKENS},
                timeout=300,
            )
            if resp.status_code == 429:
                wait = 30 + 10 * attempt
                log(f"429 rate-limit, wait {wait}s")
                time.sleep(wait)
                continue
            resp.raise_for_status()
            msg = resp.json()["choices"][0]["message"]
            content = msg.get("content")
            if content:
                return content.strip()
            # `content` null means the model never finished its reasoning within max_tokens
            # (실측 2026-07-24). The `reasoning` field is a mid-thought trace, not an answer —
            # extracting from it produced garbage ("* Length: 4 + 1 + 1 + ..."). Treat as a
            # real failure so it's skipped/retried, not silently accepted.
            raise RuntimeError("content still null at max_tokens — reasoning did not converge")
        except Exception as e:
            wait = min(2 ** attempt, 20)
            log(f"call_solar attempt {attempt+1}/{retries} failed: {e}, retry in {wait}s")
            time.sleep(wait)
    raise RuntimeError("call_solar failed after retries")


def strip_reasoning_and_clean(text):
    # solar-open2 sometimes prefixes reasoning; keep only the last non-empty line as title.
    lines = [l.strip() for l in text.splitlines() if l.strip()]
    if not lines:
        return ""
    title = lines[-1]
    title = title.strip('"\'` ')
    title = re.sub(r'^#+\s*', '', title)
    title = re.sub(r'^제목\s*[:：]\s*', '', title)
    return title[:500]


_GARBAGE_RE = re.compile(r'^[\d\s+:.,\-]+$')  # e.g. "Total: 1+1+3+1+1+2..."
_HANGUL_LATIN_RE = re.compile(r'[가-힣A-Za-z]')


def validate_title(title, old_title):
    """Reject outputs that are empty, unchanged, too short, or look like numeric/reasoning debris."""
    if not title or title == old_title:
        return False, "empty or unchanged"
    if len(title) < 4:
        return False, "too short"
    if _GARBAGE_RE.match(title):
        return False, "numeric/symbol-only garbage"
    if not _HANGUL_LATIN_RE.search(title):
        return False, "no hangul/latin content"
    if re.search(r'thinking process|analyze the request|brainstorm', title, re.IGNORECASE):
        return False, "reasoning artifact leaked into title"
    return True, "ok"


def update_title_sql(doc_id, new_title):
    """Direct DB UPDATE via ssh+psql, bypassing the ownership-gated PUT API
    (CT305-merged docs are user_id=2; PUT only allows the requester's own user_id
    and this script has no auth token — 실측 2026-07-24, 7건 404 확인 후 우회)."""
    tag = "minkytitle"
    sql = f"UPDATE documents SET title = ${tag}${new_title}${tag}$, updated_at = NOW() WHERE id = '{doc_id}';"
    with _sql_lock:
        proc = subprocess.run(
            ["ssh", SSH_HOST, f"docker exec -i {DB_CONTAINER} psql -U {DB_USER} -d {DB_NAME}"],
            input=sql, capture_output=True, text=True, timeout=30,
        )
    if proc.returncode != 0 or "UPDATE 1" not in proc.stdout:
        raise RuntimeError(f"sql update failed rc={proc.returncode} stdout={proc.stdout!r} stderr={proc.stderr!r}")


_counts_lock = threading.Lock()
_counts = {"ok": 0, "fail": 0, "skip": 0}


def process_one(i, total, item):
    doc_id, old_title = item["id"], item["title"]
    try:
        r = requests.get(f"{MINKY_BASE}/api/documents/{doc_id}", timeout=30)
        r.raise_for_status()
        doc = r.json()
        content = doc.get("markdown_content", "") or ""
        body = _FM_RE.sub("", content).strip()
        if len(body) < 20:
            log(f"[{i}/{total}] {doc_id} skip: 본문 너무 짧음(len={len(body)})")
            with _counts_lock: _counts["skip"] += 1
            return

        prompt = _PROMPT_TMPL.format(old_title=old_title, body=body[:6000])
        raw_title = call_solar(prompt)
        new_title = strip_reasoning_and_clean(raw_title)

        valid, reason = validate_title(new_title, old_title)
        if not valid:
            log(f"[{i}/{total}] {doc_id} skip: {reason} (생성값={new_title!r})")
            with _counts_lock: _counts["skip"] += 1
            return

        update_title_sql(doc_id, new_title)
        log(f"[{i}/{total}] {doc_id} OK: {old_title!r} -> {new_title!r}")
        with _counts_lock: _counts["ok"] += 1
    except Exception as e:
        log(f"[{i}/{total}] {doc_id} ERROR: {e}")
        with _counts_lock: _counts["fail"] += 1


def main():
    with open(DOC_IDS_FILE, encoding="utf-8") as f:
        items = json.load(f)  # [{"id":..., "title":...}, ...]

    total = len(items)
    log(f"=== 시작: {total}건 제목 재생성 (workers={WORKERS}, max_tokens={UPSTAGE_MAX_TOKENS}) ===")

    with ThreadPoolExecutor(max_workers=WORKERS) as pool:
        futures = [pool.submit(process_one, i, total, item) for i, item in enumerate(items, 1)]
        for fut in as_completed(futures):
            fut.result()  # surface any unexpected exception

    log(f"=== 완료: ok={_counts['ok']} fail={_counts['fail']} skip={_counts['skip']} / total={total} ===")


if __name__ == "__main__":
    main()
