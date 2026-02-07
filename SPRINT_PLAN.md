# Minky - Markdown Document Management System Sprint Plan

## Sprint 1-12: 완료
(이전 스프린트 내용 생략 - 모두 완료됨)

---

## Sprint 13 (품질 강화 및 문서화) - Current

### Goals
- 프론트엔드 테스트 커버리지 60%+ 달성
- 대형 컴포넌트 리팩토링 완료
- 백엔드 API 검증 및 에러 처리 강화
- 프로젝트 문서화 완성

### Sprint 13 Tasks

#### 1. 테스트 커버리지 확대 [P0]
- [ ] 페이지 컴포넌트 테스트 추가
  - [ ] DocumentList.test.js
  - [ ] DocumentView.test.js
  - [ ] DocumentCreate.test.js
  - [ ] DocumentEdit.test.js
- [ ] 커스텀 훅 테스트 추가
  - [ ] useAsync.test.js
  - [ ] useTagSuggestions.test.js
- [ ] 서비스 테스트 확대
  - [ ] api.test.js 확장 (실제 API 호출 모킹)

#### 2. 컴포넌트 리팩토링 [P0]
- [ ] OCRUpload.js (460줄) 분리
  - [ ] OCRPreview.js - 미리보기 컴포넌트
  - [ ] OCRProgress.js - 진행 상태 컴포넌트
  - [ ] OCRResult.js - 결과 표시 컴포넌트
- [ ] AdminPanel.js (386줄) 분리
  - [ ] UserManagement.js - 사용자 관리
  - [ ] SystemSettings.js - 시스템 설정
  - [ ] ActivityLog.js - 활동 로그

#### 3. 백엔드 개선 [P1]
- [ ] Pydantic 스키마 확장
  - [ ] DocumentSchema 추가
  - [ ] CategorySchema 추가
- [ ] 에러 처리 표준화
  - [ ] 커스텀 예외 클래스 정의
  - [ ] 일관된 에러 응답 포맷
- [ ] API 응답 캐싱 최적화
  - [ ] 자주 조회되는 엔드포인트 캐싱

#### 4. 문서화 [P1]
- [ ] 아키텍처 문서 작성
  - [ ] Docs/ARCHITECTURE.md
  - [ ] 시스템 구성도
  - [ ] 데이터 흐름도
- [ ] API 문서 업데이트
  - [ ] 새 엔드포인트 추가
  - [ ] 요청/응답 예시 보강
- [ ] 기여 가이드라인
  - [ ] CONTRIBUTING.md
  - [ ] 코드 스타일 가이드

### Acceptance Criteria
- 프론트엔드 테스트 60개+ 추가 (총 150개+)
- 400줄 이상 컴포넌트 0개
- 백엔드 모든 라우트에 Pydantic 검증
- 아키텍처 문서 완성

### Deliverables
- [ ] 프론트엔드 테스트 커버리지 60%+
- [ ] 리팩토링된 컴포넌트 구조
- [ ] 강화된 API 검증 시스템
- [ ] 완성된 프로젝트 문서

---

## Sprint History (완료)

### Sprint 12 - 프론트엔드 테스팅 및 리팩토링 ✅
- 프론트엔드 테스트 인프라 구축 (87개 테스트)
- Settings, DocumentClustering, MLAnalytics 컴포넌트 분리
- E2E 테스트 환경 구성 (Playwright)
- ErrorBoundary, LoadingSpinner 컴포넌트 추가

### Sprint 11 - Tree Navigation & Quality ✅
- Document Tree API 구현
- TreeView, TreeSidebar 컴포넌트
- 백엔드 테스트 확장 (109개)

### Sprint 10 - Pydantic & Rate Limiting ✅
- Pydantic 검증 스키마 추가
- API Rate Limiting 구현
- 보안 강화

### Sprint 9 - UI/UX 재구성 ✅
- 3-tier 네비게이션 (Documents/Explore/Config)
- 기능별 논리적 그룹핑

### Sprint 8 - 한국어 지원 & Org-roam ✅
- 한국어 형태소 분석 (KoNLPy)
- Org-roam 문서 파서
- OpenSearch 연동

### Sprint 7 - AI & Advanced Integrations ✅
- AI 기반 컨텐츠 제안
- 실시간 협업 편집
- OCR 기능
- ML 기반 문서 분석

### Sprint 1-6 ✅
- 기본 CRUD, 검색, 에디터
- 인증, 보안, 태깅
- 버전 관리, 내보내기
- 분석 대시보드
