import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { archiveService } from '../services/api';
import './DuplicatesPage.css';

export default function DuplicatesPage() {
  const [groups, setGroups] = useState([]);
  const [totalRemovable, setTotalRemovable] = useState(0);
  const [loading, setLoading] = useState(true);
  const [resolving, setResolving] = useState(false);
  const [error, setError] = useState(null);
  const [actionMsg, setActionMsg] = useState(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await archiveService.getDuplicates();
      // Backend sends document_ids + titles as parallel arrays (both ordered by created_at ASC).
      // Zip them into objects so the template can iterate naturally. All docs are active
      // (archived = false) because list_duplicates filters archived = false server-side.
      const normalized = (data.groups || []).map((g) => ({
        ...g,
        documents: (g.document_ids || []).map((id, i) => ({
          id,
          title: g.titles[i] ?? '(제목 없음)',
          archived: false,
        })),
      }));
      setGroups(normalized);
      setTotalRemovable(data.total_removable ?? 0);
    } catch (e) {
      setError(e.response?.data?.error || e.message || '로딩 실패');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const handleArchive = async (id, title) => {
    try {
      await archiveService.archiveDocument(id);
      setActionMsg(`"${title}" 아카이브 완료`);
      await load();
    } catch (e) {
      setActionMsg(`오류: ${e.response?.data?.error || e.message}`);
    }
  };

  const handleResolveAll = async () => {
    if (!window.confirm(`${totalRemovable}건을 아카이브하겠습니까? (각 그룹에서 가장 오래된 버전만 남깁니다)`)) return;
    setResolving(true);
    setActionMsg(null);
    try {
      const result = await archiveService.resolveDuplicates();
      setActionMsg(`완료: ${result.archived_count}건 아카이브, ${result.groups_resolved}그룹 처리`);
      await load();
    } catch (e) {
      setActionMsg(`오류: ${e.response?.data?.error || e.message}`);
    } finally {
      setResolving(false);
    }
  };

  return (
    <div className="duplicates-page">
      <div className="page-header">
        <h1>중복 문서 관리</h1>
        <div className="header-actions">
          {groups.length > 0 && (
            <button
              className="btn-resolve-all"
              onClick={handleResolveAll}
              disabled={resolving}
            >
              {resolving ? '처리 중...' : `전체 정리 (${totalRemovable}건 아카이브)`}
            </button>
          )}
          <Link to="/archived" className="archived-link">
            아카이브 목록 →
          </Link>
        </div>
      </div>

      {actionMsg && (
        <div className="action-msg" onClick={() => setActionMsg(null)}>
          {actionMsg}
        </div>
      )}

      {loading && <div className="loading">로딩 중...</div>}
      {error && <div className="error-msg">{error}</div>}

      {!loading && !error && groups.length === 0 && (
        <div className="empty-state">중복 문서가 없습니다.</div>
      )}

      {groups.map((group, gi) => (
        <div key={gi} className="dup-group">
          <div className="group-header">
            <span className="group-count">중복 {group.documents.length}건</span>
            <span className="group-hash">해시: {group.content_hash?.slice(0, 12)}…</span>
          </div>
          <ul className="doc-list">
            {group.documents.map((doc) => (
              <li key={doc.id} className="doc-item">
                <div className="doc-info">
                  <Link to={`/documents/${doc.id}`} className="doc-title">
                    {doc.title}
                  </Link>
                </div>
                <div className="doc-actions">
                  <button
                    className="btn-archive"
                    onClick={() => handleArchive(doc.id, doc.title)}
                  >
                    아카이브
                  </button>
                </div>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}
