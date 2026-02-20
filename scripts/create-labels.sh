#!/bin/bash
# GitHub 라벨 생성 스크립트
# 사용법: ./scripts/create-labels.sh

set -e

echo "🏷️  Creating GitHub labels..."

# Priority labels
echo "Creating priority labels..."
gh label create "priority:critical" --color "B60205" --description "즉시 해결 필요" --force 2>/dev/null || true
gh label create "priority:high" --color "D93F0B" --description "빠른 해결 필요" --force 2>/dev/null || true
gh label create "priority:medium" --color "FBCA04" --description "일반 우선순위" --force 2>/dev/null || true
gh label create "priority:low" --color "0E8A16" --description "낮은 우선순위" --force 2>/dev/null || true

# Type labels
echo "Creating type labels..."
gh label create "type:bug" --color "D73A4A" --description "버그 수정" --force 2>/dev/null || true
gh label create "type:feature" --color "0075CA" --description "새 기능" --force 2>/dev/null || true
gh label create "type:enhancement" --color "A2EEEF" --description "개선" --force 2>/dev/null || true
gh label create "type:docs" --color "0075CA" --description "문서" --force 2>/dev/null || true
gh label create "type:refactor" --color "7057FF" --description "리팩토링" --force 2>/dev/null || true

# Status labels
echo "Creating status labels..."
gh label create "status:needs-triage" --color "FBCA04" --description "분류 필요" --force 2>/dev/null || true
gh label create "status:pm-reviewed" --color "5319E7" --description "PM 검토 완료" --force 2>/dev/null || true
gh label create "status:in-progress" --color "1D76DB" --description "작업 중" --force 2>/dev/null || true
gh label create "status:ready-for-review" --color "0E8A16" --description "리뷰 대기" --force 2>/dev/null || true
gh label create "status:blocked" --color "B60205" --description "차단됨" --force 2>/dev/null || true

# Area labels
echo "Creating area labels..."
gh label create "area:backend" --color "1D76DB" --description "백엔드" --force 2>/dev/null || true
gh label create "area:frontend" --color "7057FF" --description "프론트엔드" --force 2>/dev/null || true
gh label create "area:infra" --color "0E8A16" --description "인프라" --force 2>/dev/null || true
gh label create "area:docs" --color "0075CA" --description "문서" --force 2>/dev/null || true

echo ""
echo "✅ Labels created successfully!"
echo ""
echo "현재 라벨 목록:"
gh label list
