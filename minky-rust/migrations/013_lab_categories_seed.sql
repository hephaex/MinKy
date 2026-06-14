-- Sprint 2: lab 카테고리 시딩 (parent + 9 children)
-- user_id=1 (lab user), ON CONFLICT DO NOTHING = 멱등 실행
-- Fix E.1: SELECT FROM users WHERE id=1 → user 없는 신규 DB에서 FK 위반 방지

-- 1. lab 부모 카테고리
INSERT INTO categories (name, parent_id, user_id, slug, description, color, sort_order, is_active)
SELECT 'lab', NULL, u.id, 'lab', 'Lablup CNE 팀 문서 허브', '#6366f1', 0, true
FROM users u WHERE u.id = 1
ON CONFLICT (slug) WHERE slug IS NOT NULL DO NOTHING;

-- 2. 9개 자식 카테고리 (lab parent 아래)
INSERT INTO categories (name, parent_id, user_id, slug, description, color, sort_order, is_active)
SELECT child.name, p.id, sys_user.id, child.slug, child.description, child.color, child.sort_order, true
FROM (VALUES
    ('finance',    'lab-finance',    '재무제표·감사·견적·계약',      '#ef4444', 1),
    ('hr',         'lab-hr',         '채용·평가·급여·조직',          '#f97316', 2),
    ('sales',      'lab-sales',      '제안서·PoC결과·고객미팅',      '#eab308', 3),
    ('research',   'lab-research',   '기술조사·논문·벤치마크',        '#22c55e', 4),
    ('core',       'lab-core',       '핵심 제품·아키텍처·인프라',     '#06b6d4', 5),
    ('frontend',   'lab-frontend',   '프론트엔드·UI·사용자경험',      '#3b82f6', 6),
    ('cne-infra',  'lab-cne-infra',  'CNE 인프라·DevOps·배포',       '#8b5cf6', 7),
    ('product',    'lab-product',    '제품전략·로드맵·릴리즈노트',    '#ec4899', 8),
    ('legal',      'lab-legal',      '법무·계약서·NDA·특허',         '#6b7280', 9)
) AS child(name, slug, description, color, sort_order)
JOIN categories p ON p.slug = 'lab'
CROSS JOIN (SELECT id FROM users WHERE id = 1) AS sys_user
ON CONFLICT (slug) WHERE slug IS NOT NULL DO NOTHING;
