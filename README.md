# Minky - 마크다운 문서 관리 시스템

PostgreSQL에서 마크다운 문서를 저장, 목록화, 검색할 수 있는 웹 서비스입니다.

## 기능

- 자동 HTML 변환을 통한 마크다운 문서 저장
- 페이지네이션을 지원하는 문서 목록
- 제목과 내용으로 문서 검색
- REST API를 통한 완전한 CRUD 작업
- PostgreSQL 데이터베이스 저장

## 설정

1. 의존성 설치:
```bash
pip install -r requirements.txt
```

2. PostgreSQL 데이터베이스 설정 및 환경 구성:
```bash
cp .env.example .env
# 데이터베이스 자격 증명으로 .env 파일 편집
```

3. 데이터베이스 초기화:
```bash
flask db init
flask db migrate -m "Initial migration"
flask db upgrade
```

4. 애플리케이션 실행:
```bash
python run.py
```

## API 엔드포인트

### 인증
- `POST /api/auth/register` - 새 사용자 등록
- `POST /api/auth/login` - 사용자 로그인
- `POST /api/auth/refresh` - 액세스 토큰 갱신
- `GET /api/auth/me` - 현재 사용자 프로필 조회
- `PUT /api/auth/profile` - 사용자 프로필 업데이트

### 문서
- `POST /api/documents` - 새 문서 생성 (태그 포함)
- `GET /api/documents` - 문서 목록 조회 (페이지네이션, 검색, 태그 필터링, 비공개 문서 지원)
- `GET /api/documents/{id}` - 특정 문서 조회
- `PUT /api/documents/{id}` - 문서 업데이트 (변경 추적 포함)
- `DELETE /api/documents/{id}` - 문서 삭제

### 태그
- `GET /api/tags` - 모든 태그 목록 조회 (인기도 순 정렬)
- `POST /api/tags` - 새 태그 생성
- `GET /api/tags/{slug}` - 태그 세부 정보 및 관련 문서 조회
- `PUT /api/tags/{slug}` - 태그 업데이트
- `DELETE /api/tags/{slug}` - 태그 삭제
- `GET /api/tags/suggest` - 태그 제안 조회

### 댓글 및 평가
- `GET /api/documents/{id}/comments` - 문서의 댓글 조회
- `POST /api/documents/{id}/comments` - 댓글 추가 (중첩 답글 지원)
- `PUT /api/comments/{id}` - 댓글 업데이트
- `DELETE /api/comments/{id}` - 댓글 삭제
- `POST /api/documents/{id}/rating` - 문서 평가 (1-5점)
- `GET /api/documents/{id}/rating` - 문서 평가 통계 조회
- `DELETE /api/documents/{id}/rating` - 사용자 평가 삭제

### 버전 관리
- `GET /api/documents/{id}/versions` - 버전 히스토리 조회
- `GET /api/documents/{id}/versions/{version}` - 특정 버전 조회
- `POST /api/documents/{id}/versions/{version}/restore` - 특정 버전으로 복원
- `GET /api/documents/{id}/versions/{version}/diff` - 이전 버전과의 차이 조회
- `GET /api/documents/{id}/versions/compare` - 두 버전 비교
- `GET /api/documents/{id}/snapshots` - 문서 스냅샷 조회

### 템플릿
- `GET /api/templates` - 모든 공개 템플릿 목록 조회 (필터링 및 카테고리 지원)
- `POST /api/templates` - 새 템플릿 생성
- `GET /api/templates/{id}` - 템플릿 세부 정보 조회
- `PUT /api/templates/{id}` - 템플릿 업데이트
- `DELETE /api/templates/{id}` - 템플릿 삭제
- `POST /api/templates/{id}/create-document` - 템플릿으로 문서 생성
- `POST /api/templates/{id}/preview` - 변수로 템플릿 미리보기
- `GET /api/templates/categories` - 템플릿 카테고리 조회
- `GET /api/my-templates` - 현재 사용자의 템플릿 조회

### 파일 첨부
- `POST /api/attachments/upload` - 파일 업로드
- `GET /api/attachments/{id}/download` - 파일 다운로드
- `GET /api/attachments/{id}/preview` - 파일 미리보기 (이미지, 동영상)
- `GET /api/attachments` - 사용자 첨부파일 목록 조회
- `GET /api/attachments/{id}` - 첨부파일 세부 정보 조회
- `DELETE /api/attachments/{id}` - 첨부파일 삭제
- `GET /api/documents/{id}/attachments` - 문서 첨부파일 조회
- `GET /api/attachments/stats` - 첨부파일 통계 조회

### 문서 내보내기
- `GET /api/documents/{id}/export/{format}` - 단일 문서 내보내기 (html, pdf, docx, markdown, json)
- `POST /api/documents/bulk-export` - 여러 문서를 ZIP으로 내보내기
- `GET /api/documents/{id}/export/bundle` - 문서를 여러 형식으로 ZIP 내보내기
- `GET /api/export/formats` - 지원하는 내보내기 형식 조회

### 알림
- `GET /api/notifications` - 사용자 알림 조회 (페이지네이션 및 필터링 지원)
- `POST /api/notifications/{id}/read` - 알림 읽음 처리
- `POST /api/notifications/read-all` - 모든 알림 읽음 처리
- `DELETE /api/notifications/{id}` - 알림 삭제
- `GET /api/notifications/summary` - 알림 요약 및 개수 조회
- `GET /api/notifications/preferences` - 알림 설정 조회
- `PUT /api/notifications/preferences` - 알림 설정 업데이트
- `POST /api/notifications/bulk-actions` - 알림 대량 작업 수행
- `POST /api/notifications/test` - 테스트 알림 생성 (개발용)

### 문서 워크플로우
- `GET /api/documents/{id}/workflow` - 문서의 워크플로우 정보 조회
- `POST /api/documents/{id}/workflow/action` - 워크플로우 작업 수행 (제출, 승인, 거부 등)
- `GET /api/workflows/pending` - 현재 사용자의 검토 대기 워크플로우 조회
- `GET /api/workflow-templates` - 사용 가능한 워크플로우 템플릿 조회
- `POST /api/workflow-templates` - 워크플로우 템플릿 생성 (관리자만)
- `PUT /api/workflow-templates/{id}` - 워크플로우 템플릿 업데이트 (관리자만)
- `POST /api/documents/{id}/workflow/assign-template/{template_id}` - 문서에 워크플로우 템플릿 할당
- `GET /api/workflows/stats` - 현재 사용자의 워크플로우 통계 조회

### 한국어 검색
- `POST /api/search/korean` - 한국어 텍스트 전용 검색 (형태소 분석 지원)
- `GET /api/search/suggest-tags` - 한국어 태그 자동완성
- `POST /api/documents/{id}/analyze-korean` - 문서의 한국어 분석 (키워드, 가독성 등)
- `GET /api/search/statistics` - 한국어 검색 통계
- `GET /api/search/health` - 검색 시스템 상태 확인

### Org-roam 연동 (Emacs 통합)
- `POST /api/org-roam/upload` - org-roam 파일 업로드 (단일/ZIP)
- `POST /api/org-roam/import-directory` - 서버 디렉토리에서 org-roam 임포트 (관리자)
- `GET /api/org-roam/documents` - org-roam에서 임포트된 문서 목록
- `GET /api/org-roam/documents/{id}/links` - 문서 링크 관계 (백링크/아웃바운드)
- `GET /api/org-roam/statistics` - org-roam 임포트 통계

### 보안 및 모니터링 (관리자만)
- `GET /api/security/status` - 전체 보안 상태 및 메트릭 조회
- `GET /api/security/logs` - 최근 보안 이벤트 및 로그 조회
- `GET /api/security/threats` - 위협 분석 및 차단된 IP 조회
- `GET /api/security/config` - 현재 보안 설정 조회
- `PUT /api/security/config` - 보안 설정 업데이트
- `POST /api/security/ip-management` - 허용/차단 목록에 IP 추가/제거
- `POST /api/security/scan` - 종합 보안 스캔 실행

### 상태 모니터링
- `GET /api/health` - 기본 상태 확인
- `GET /api/health/detailed` - 자세한 상태 정보
- `GET /metrics` - Prometheus 메트릭 (성능 모니터링)

### 전문 검색 (Full-Text Search)
- `GET /api/documents/search?q=검색어` - PostgreSQL 전문 검색 (제목, 내용 통합 검색)

### API 문서
- `GET /api/docs/` - Swagger UI 인터랙티브 API 문서
- `GET /api/docs/apispec.json` - OpenAPI JSON 스펙

## 환경 변수

| 변수명 | 설명 | 기본값 |
|--------|------|--------|
| `DATABASE_URL` | PostgreSQL 연결 문자열 | `postgresql://localhost/minky_db` |
| `SECRET_KEY` | Flask 세션 암호화 키 | (개발: dev-secret-key) |
| `JWT_SECRET_KEY` | JWT 토큰 서명 키 | (개발: jwt-secret-key) |
| `FLASK_ENV` | 실행 환경 (development/production) | `production` |
| `CORS_ORIGINS` | 허용된 CORS 도메인 (콤마 구분) | `http://localhost:3000` |
| `REDIS_URL` | Redis 연결 문자열 (캐싱/속도제한) | `memory://` |
| `CACHE_TYPE` | 캐시 백엔드 (SimpleCache/RedisCache) | `SimpleCache` |
| `CACHE_DEFAULT_TIMEOUT` | 캐시 만료 시간 (초) | `300` |
| `METRICS_ENABLED` | Prometheus 메트릭 활성화 | `true` |
| `LOG_LEVEL` | 로그 레벨 (DEBUG/INFO/WARNING/ERROR) | `INFO` |
| `LOG_FORMAT` | 로그 형식 (json/text) | `json` |
| `RATE_LIMIT_DEFAULT` | 기본 API 속도 제한 | `1000 per hour` |

## 테스트

테스트 실행:
```bash
pytest tests/ -v --cov=app
```

커버리지 리포트 포함:
```bash
pytest tests/ --cov=app --cov-report=html
```

## 개발 환경 설정

### 백엔드 설정
1. 의존성 설치:
```bash
pip install -r requirements.txt
```

2. PostgreSQL 데이터베이스 설정:
```bash
createdb minky_db
cp .env.example .env
# 데이터베이스 자격 증명으로 .env 파일 편집
```

3. 데이터베이스 초기화:
```bash
export FLASK_APP=run.py
flask db init
flask db migrate -m "Initial migration"
flask db upgrade
```

4. 백엔드 서버 실행:
```bash
python run.py
```

### 프론트엔드 설정
1. 프론트엔드 디렉토리로 이동:
```bash
cd frontend
```

2. 의존성 설치:
```bash
npm install
```

3. 개발 서버 시작:
```bash
npm start
```

프론트엔드는 http://localhost:3000 에서 실행되며, API 요청을 http://localhost:5000 의 백엔드로 프록시합니다.

## 프로덕션 배포

### Docker 사용 (권장)

1. 리포지토리 클론:
```bash
git clone <repository-url>
cd minky
```

2. 배포 스크립트 실행:
```bash
./deploy.sh
```

다음 작업을 수행합니다:
- 모든 서비스의 Docker 이미지 빌드
- 적절한 인덱스로 PostgreSQL 데이터베이스 설정
- Redis 캐시 서버 설정
- 데이터베이스 마이그레이션 실행
- 모든 서비스 시작 (프론트엔드, 백엔드, 데이터베이스, Redis)

#### Docker Compose 서비스 구성
```yaml
services:
  - backend (Flask API, 포트 5000)
  - frontend (React, 포트 3000)
  - db (PostgreSQL, 포트 5432)
  - redis (Redis 캐시, 포트 6379)
```

### 수동 배포

1. 환경 변수 설정:
```bash
cp .env.production .env
# 프로덕션 설정으로 .env 파일 편집
```

2. PostgreSQL 데이터베이스 설정:
```bash
createdb minky_production
```

3. 마이그레이션 실행 및 인덱스 생성:
```bash
export FLASK_APP=run.py
flask db upgrade
python app/utils/performance.py
```

4. 애플리케이션 시작:
```bash
gunicorn --bind 0.0.0.0:5000 --workers 4 run:app
```

## 보안 기능

- **인증**: 리프레시 토큰을 사용한 JWT 기반 인증
- **권한 부여**: 문서에 대한 역할 기반 접근 제어
- **XSS 방지**: bleach 라이브러리를 사용한 입력 살균
- **CSRF 보호**: Flask-WTF를 통한 보호
- **SQL 인젝션 방지**: 매개변수화된 쿼리를 사용한 SQLAlchemy ORM
- **비밀번호 보안**: 솔트를 사용한 Bcrypt 해싱
- **입력 검증**: 이메일 검증 및 비밀번호 복잡성 요구사항
- **보안 헤더**: Flask-Talisman을 통한 CSP, HSTS, X-Frame-Options
- **속도 제한**: Flask-Limiter를 통한 API 속도 제한 (Redis 백엔드 지원)
- **환경별 보안**: 프로덕션 환경에서 강력한 시크릿 키 강제

## 성능 최적화

- **데이터베이스 인덱스**: 검색 및 필터링을 위한 최적화된 인덱스
- **전문 검색**: 빠른 텍스트 검색을 위한 PostgreSQL GIN 인덱스
- **응답 캐싱**: Flask-Caching을 통한 API 응답 캐싱 (Redis/SimpleCache)
- **압축**: 모든 텍스트 기반 응답에 대한 Gzip 압축
- **연결 풀링**: SQLAlchemy를 통한 데이터베이스 연결 풀링
- **메트릭 모니터링**: Prometheus를 통한 실시간 성능 메트릭

## 모니터링

### Prometheus 메트릭
`/metrics` 엔드포인트에서 다음 메트릭을 제공합니다:
- HTTP 요청 수 및 지연 시간
- 응답 상태 코드 분포
- 애플리케이션 정보

### 구조화된 로깅
JSON 형식의 로그 출력 (ELK/Loki 호환):
```json
{
  "timestamp": "2024-01-01T00:00:00Z",
  "level": "INFO",
  "logger": "minky.access",
  "message": "Request completed",
  "request_id": "abc-123",
  "method": "GET",
  "path": "/api/documents",
  "duration_seconds": 0.045,
  "status_code": 200
}
```

## 스프린트 구현 현황

### ✅ 스프린트 1 (기초 및 핵심 백엔드)
- PostgreSQL을 사용한 Flask 프로젝트 구조
- 완전한 CRUD API 엔드포인트
- 마크다운에서 HTML 변환
- 기본 검색 기능
- 페이지네이션 지원
- 단위 테스트

### ✅ 스프린트 2 (검색 및 프론트엔드 기초)
- 랭킹을 지원하는 향상된 PostgreSQL 전문 검색
- React 프론트엔드 애플리케이션
- 검색 및 페이지네이션을 지원하는 문서 목록 페이지
- 마크다운 렌더링을 지원하는 문서 보기 페이지
- 반응형 디자인 및 스타일링
- API 통합을 위한 CORS 지원

### ✅ 스프린트 3 (편집기 및 향상된 기능)
- 실시간 미리보기를 지원하는 통합 고급 마크다운 편집기
- 검증을 지원하는 문서 생성 및 편집 폼
- 스마트 발췌를 지원하는 검색 결과 강조
- 모바일 장치를 위한 향상된 반응형 디자인
- 더 나은 네비게이션으로 개선된 사용자 경험
- 분할 보기 모드를 지원하는 고급 편집기 기능

### ✅ 스프린트 4 (보안 및 완성도)
- JWT 토큰을 사용한 완전한 사용자 인증 시스템
- 문서에 대한 역할 기반 권한 제어
- 입력 살균을 통한 XSS 방지
- 인덱스를 통한 데이터베이스 성능 최적화
- 프로덕션 준비 완료 배포 구성
- docker-compose를 사용한 Docker 컨테이너화
- 상태 모니터링 엔드포인트
- 종합적인 보안 조치

### ✅ 스프린트 5 (향후 개선사항)
- 태그 기반 필터링 및 검색을 지원하는 완전한 태깅 시스템
- 중첩 댓글을 지원하는 댓글 및 평가 기능
- 차이 생성 및 복원 기능을 지원하는 문서 버전 관리
- 향상된 마크다운 지원을 포함한 고급 편집기 기능
- 효율적인 버전 관리를 위한 문서 스냅샷
- 문서 상호작용을 위한 커뮤니티 기능
- 향상된 문서 구성 및 발견

### ✅ 스프린트 6 (고급 기능 및 분석)
- 변수 치환 및 카테고리를 지원하는 문서 템플릿 라이브러리
- 미디어 미리보기 및 썸네일을 지원하는 파일 첨부 시스템
- 사용 통계 및 인사이트를 지원하는 분석 대시보드
- 고급 협업 기능 및 문서 공유
- 여러 필터 옵션을 지원하는 향상된 검색
- 시스템 관리를 위한 종합적인 관리자 패널
- 자동 썸네일 생성을 지원하는 미디어 관리
- 사용자 정의 변수를 지원하는 템플릿 기반 문서 생성

### ⚡ 스프린트 7 (AI 및 고급 통합) - 진행 중
- 여러 형식(HTML, PDF, DOCX, Markdown, JSON)을 지원하는 문서 내보내기 시스템
- 문서 활동 및 사용자 상호작용을 위한 종합적인 알림 시스템
- 속도 제한, 위협 탐지, 모니터링을 지원하는 고급 API 보안
- 승인 과정 및 사용자 정의 템플릿을 지원하는 문서 워크플로우 관리
- 입력 검증 및 의심스러운 패턴 탐지를 지원하는 보안 미들웨어
- 위협 분석 및 구성 관리를 지원하는 관리자 보안 대시보드

### ✅ 스프린트 8 (한국어 지원 및 Org-roam 연동)
- 한국어 형태소 분석기 통합 (KoNLPy/Mecab) 및 전문 검색 시스템
- Emacs org-roam 문서 완전 호환성 (파싱, 임포트, 메타데이터 처리)
- OpenSearch 기반 다국어 검색 엔진 (한국어 분석기 포함)
- 문서 간 링크 관계 시스템 (백링크, 아웃바운드 링크, 관계 시각화)
- 한국어 문서 자동 태깅 및 키워드 추출
- org-roam 메타데이터 보존 (ID, 태그, 별칭, 링크 구조)

### ✅ 스프린트 9 (API 문서화 및 보안 강화)
- Swagger/OpenAPI 인터랙티브 API 문서 (`/api/docs/`)
- Flask-Talisman을 통한 보안 헤더 (CSP, HSTS, X-Frame-Options)
- Flask-Caching을 통한 응답 캐싱 (Redis/SimpleCache 지원)
- Prometheus 메트릭 엔드포인트 (`/metrics`)
- 민감한 엔드포인트에 대한 속도 제한 강화

### ✅ 스프린트 10 (DevOps 및 프론트엔드 개선)
- Docker Compose에 Redis 서비스 추가
- GitHub Actions CI/CD 파이프라인 (ruff, mypy, pytest, codecov)
- React 에러 바운더리 및 로딩 상태 컴포넌트
- useAsync 커스텀 훅을 통한 비동기 상태 관리
- SQLAlchemy 2.0 호환성 마이그레이션

### ✅ 스프린트 11 (모니터링 및 로깅)
- python-json-logger를 통한 구조화된 JSON 로깅
- 요청 ID 추적 (X-Request-ID 헤더)
- 요청 기간 및 상태 코드 로깅
- ELK/Loki 로그 집계 호환 형식
- 테스트 환경에서 메트릭 충돌 방지
