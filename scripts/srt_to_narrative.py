#!/usr/bin/env python3
"""
SRT 자막 파일 → Solar-Open2 서술형 글 변환기

사용법:
  cd ~/Simon/minky && scripts/srt_to_narrative.py input.srt [output.md]

  UPSTAGE_API_KEY 또는 LLM_API_KEY 를 ~/Simon/minky/.env 에 저장하면
  매 실행마다 키를 지정할 필요가 없습니다.

환경변수 (우선순위 높은 순):
  UPSTAGE_API_KEY  Upstage API 키 (권장)
  LLM_API_KEY      위 키가 없을 때 폴백
  SOLAR_MODEL      모델 ID (기본: solar-open2-260528)
  CHUNK_SIZE       청크 크기 글자수 (기본: 2500)
"""

import os
import re
import sys
import time
from pathlib import Path

import requests

# .env 자동 로드: 스크립트 기준 상위 디렉토리(프로젝트 루트)를 탐색
def _load_dotenv() -> None:
    """표준 dotenv 형식의 .env 파일을 os.environ에 주입.

    프로젝트 .env가 쉘 환경보다 우선됩니다 (만료된 전역 키 방지).
    """
    search = Path(__file__).resolve().parent
    for _ in range(3):
        candidate = search / ".env"
        if candidate.is_file():
            with open(candidate, encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if not line or line.startswith("#") or "=" not in line:
                        continue
                    key, _, val = line.partition("=")
                    key = key.strip()
                    val = val.strip()
                    # 따옴표로 감싼 값은 # 제거 없이 따옴표만 벗김 (F6: # 포함 패스워드 보호)
                    if (val.startswith('"') and val.endswith('"')) or \
                       (val.startswith("'") and val.endswith("'")):
                        val = val[1:-1]
                    else:
                        val = val.split("#")[0].strip()
                    if key and val:  # 값이 있는 항목만 오버라이드
                        os.environ[key] = val
            break
        search = search.parent

_load_dotenv()

UPSTAGE_URL = "https://api.upstage.ai/v1/chat/completions"
MODEL = os.getenv("SOLAR_MODEL", "solar-open2")
# UPSTAGE_API_KEY 우선, LLM_API_KEY 폴백 (baram 호환)
API_KEY = os.getenv("UPSTAGE_API_KEY") or os.getenv("LLM_API_KEY", "")
CHUNK_SIZE = int(os.getenv("CHUNK_SIZE", "2500"))

SYSTEM_PROMPT = """\
당신은 경제·투자 대담 자막을 서술형 글로 변환하는 전문 편집자입니다.

규칙:
- 자막 내용을 요약하지 말고, 원문의 논지·주장·수치·사례를 빠짐없이 유지하세요.
- "이제", "막", "뭐", "그니까", "어" 같은 구어 채움말은 정리하되 의미 있는 표현은 살리세요.
- 완성된 문단 형태의 문어체로 작성하세요.
- 마크다운 헤딩·리스트·강조 없이 흐르는 산문(prose)으로 작성하세요.
- 한 청크에 여러 주제가 섞이면 자연스럽게 문단을 나눠도 됩니다.\
"""


# ── SRT 파싱 ─────────────────────────────────────────────────────────────────

def parse_srt(text: str) -> str:
    """Rolling 2-line SRT 자막에서 중복 제거 후 순수 텍스트 반환.

    이 형식은 매 블록이 최근 2줄의 누적 창(rolling window)을 보여준다.
    각 블록의 마지막 비어있지 않은 줄이 '이번 블록의 새 텍스트'이므로
    이전 블록과 다를 때만 수집한다.
    """
    blocks = re.split(r'\n{2,}', text.strip())
    result: list[str] = []
    prev_line: str | None = None

    for block in blocks:
        lines = []
        for raw in block.splitlines():
            line = raw.strip()
            if not line:
                continue
            if re.match(r'^\d+$', line):          # 블록 번호
                continue
            if re.match(r'\d{2}:\d{2}:\d{2},\d+', line):  # 타임스탬프
                continue
            lines.append(line)

        if not lines:
            continue

        new_line = lines[-1]
        if new_line != prev_line:
            result.append(new_line)
            prev_line = new_line

    return '\n'.join(result)


def chunk_text(text: str, max_chars: int = CHUNK_SIZE) -> list[str]:
    """텍스트를 max_chars 글자 단위로 분할."""
    lines = text.split('\n')
    chunks: list[str] = []
    buf: list[str] = []
    buf_len = 0

    for line in lines:
        if buf_len + len(line) + 1 > max_chars and buf:
            chunks.append('\n'.join(buf))
            buf = [line]
            buf_len = len(line)
        else:
            buf.append(line)
            buf_len += len(line) + 1

    if buf:
        chunks.append('\n'.join(buf))

    return chunks


# ── Solar-Open2 API ──────────────────────────────────────────────────────────

def strip_think(text: str) -> str:
    """Solar reasoning <think> 태그 제거."""
    text = re.sub(r'<think>.*?</think>', '', text, flags=re.DOTALL).strip()
    if '<think>' in text:
        text = text[:text.find('<think>')].strip()
    return text


def call_solar(chunk: str, chunk_num: int, total: int) -> str:
    """Solar-Open2로 자막 청크를 서술형 글로 변환."""
    print(f"  [{chunk_num}/{total}] API 호출 중 ({len(chunk)}자)...", end=' ', flush=True)
    t0 = time.time()

    payload = {
        "model": MODEL,
        "max_tokens": 4096,
        "temperature": 0.3,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": f"다음 자막을 서술형 글로 작성하세요:\n\n{chunk}"},
        ],
    }

    resp = requests.post(
        UPSTAGE_URL,
        json=payload,
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
        timeout=120,
    )
    resp.raise_for_status()

    raw = resp.json()["choices"][0]["message"]["content"]
    result = strip_think(raw)
    print(f"완료 ({len(result)}자, {time.time()-t0:.1f}s)")
    return result


# ── 제목 추출 ─────────────────────────────────────────────────────────────────

def extract_title(path: str) -> str:
    """파일명에서 제목 추출.

    '제목.NA.채널명.ko.srt' 패턴 처리.
    """
    stem = Path(path).stem
    # .NA. 이전 부분을 제목으로
    if '.NA.' in stem:
        stem = stem[:stem.index('.NA.')]
    # .ko, .en 등 언어 코드 제거
    stem = re.sub(r'\.(ko|en|ja|zh)$', '', stem)
    return stem.strip()


# ── main ─────────────────────────────────────────────────────────────────────

def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: srt_to_narrative.py <input.srt> [output.md]")
        sys.exit(1)

    srt_path = sys.argv[1]
    out_path = sys.argv[2] if len(sys.argv) > 2 else str(Path(srt_path).with_suffix('')) + '_narrative.md'

    if not API_KEY:
        sys.exit("FATAL: LLM_API_KEY 환경변수를 설정하세요.")

    print(f"입력: {srt_path}")

    with open(srt_path, 'r', encoding='utf-8') as f:
        srt_content = f.read()

    clean_text = parse_srt(srt_content)
    print(f"자막 파싱 완료: {len(clean_text):,}자")

    chunks = chunk_text(clean_text, max_chars=CHUNK_SIZE)
    print(f"청크 분할: {len(chunks)}개 (청크당 최대 {CHUNK_SIZE}자)\n")

    title = extract_title(srt_path)
    narratives: list[str] = []

    for i, chunk in enumerate(chunks, 1):
        narrative = call_solar(chunk, i, len(chunks))
        narratives.append(narrative)

    # MD 조립
    md_parts = [f"# {title}\n"]
    md_parts.extend(narratives)
    md_content = '\n\n'.join(md_parts)

    with open(out_path, 'w', encoding='utf-8') as f:
        f.write(md_content)

    total_chars = sum(len(n) for n in narratives)
    print(f"\n완료 → {out_path}")
    print(f"총 {total_chars:,}자 서술문 생성 ({len(narratives)}개 문단)")


if __name__ == '__main__':
    main()
