-- Sprint 8: gyeol 메모리 동기화용 memory 카테고리 시딩
-- user_id=1 (lab user), ON CONFLICT DO NOTHING = 멱등 실행

-- 1. memory 루트 카테고리
INSERT INTO categories (name, parent_id, user_id, slug, description, color, sort_order, is_active)
SELECT 'memory', NULL, u.id, 'memory', 'gyeol 메모리 시스템 — 에피소드·시맨틱·본드·회고', '#7c3aed', 10, true
FROM users u WHERE u.id = 1
ON CONFLICT (slug) WHERE slug IS NOT NULL DO NOTHING;

-- 2. 5개 하위 카테고리
INSERT INTO categories (name, parent_id, user_id, slug, description, color, sort_order, is_active)
SELECT child.name, p.id, sys_user.id, child.slug, child.description, child.color, child.sort_order, true
FROM (VALUES
    ('episode',    'memory-episode',    '일별 세션 에피소드 로그',        '#a78bfa', 1),
    ('semantic',   'memory-semantic',   '시맨틱 레퍼런스·도메인 지식',   '#c4b5fd', 2),
    ('bond',       'memory-bond',       '사용자 관계·피드백·선호',        '#ddd6fe', 3),
    ('reflection', 'memory-reflection', '월간 회고·인사이트',             '#ede9fe', 4),
    ('reference',  'memory-reference',  '기술 레퍼런스·ADR·운영 패턴',   '#8b5cf6', 5)
) AS child(name, slug, description, color, sort_order)
JOIN categories p ON p.slug = 'memory'
CROSS JOIN (SELECT id FROM users WHERE id = 1) AS sys_user
ON CONFLICT (slug) WHERE slug IS NOT NULL DO NOTHING;
