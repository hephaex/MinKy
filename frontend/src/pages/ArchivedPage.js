import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { archiveService } from '../services/api';
import './ArchivedPage.css';

export default function ArchivedPage() {
  const [docs, setDocs] = useState([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [actionMsg, setActionMsg] = useState(null);
  const LIMIT = 50;

  const load = async (p = page) => {
    setLoading(true);
    setError(null);
    try {
      const data = await archiveService.getArchivedDocuments(p, LIMIT);
      setDocs(data.documents || []);
      setTotal(data.total || 0);
    } catch (e) {
      setError(e.response?.data?.error || e.message || '로딩 실패');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load(page);
  }, [page]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleRestore = async (id, title) => {
    try {
      await archiveService.restoreDocument(id);
      setActionMsg(`"${title}" 복원 완료`);
      await load(page);
    } catch (e) {
      setActionMsg(`오류: ${e.response?.data?.error || e.message}`);
    }
  };

  const totalPages = Math.ceil(total / LIMIT);

  return (
    <div className="archived-page">
      <div className="page-header">
        <h1>아카이브 목록</h1>
        <Link to="/duplicates" className="dup-link">
          ← 중복 관리
        </Link>
      </div>
      <div className="summary">총 {total}개 아카이브됨</div>

      {actionMsg && (
        <div
          className="action-msg"
          onClick={() => setActionMsg(null)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ' || e.key === 'Escape') {
              e.preventDefault();
              setActionMsg(null);
            }
          }}
          role="button"
          tabIndex={0}
          aria-label="알림 닫기"
        >
          {actionMsg}
        </div>
      )}

      {loading && <div className="loading">로딩 중...</div>}
      {error && <div className="error-msg">{error}</div>}

      {!loading && !error && docs.length === 0 && (
        <div className="empty-state">아카이브된 문서가 없습니다.</div>
      )}

      <ul className="archived-list">
        {docs.map((doc) => (
          <li key={doc.id} className="archived-item">
            <div className="doc-info">
              <Link to={`/documents/${doc.id}`} className="doc-title">
                {doc.title}
              </Link>
              <span className="doc-meta">
                생성: {new Date(doc.created_at).toLocaleDateString('ko-KR')}
                {doc.archived_at && (
                  <> · 아카이브: {new Date(doc.archived_at).toLocaleDateString('ko-KR')}</>
                )}
              </span>
            </div>
            <button className="btn-restore" onClick={() => handleRestore(doc.id, doc.title)}>
              복원
            </button>
          </li>
        ))}
      </ul>

      {totalPages > 1 && (
        <div className="pagination">
          <button disabled={page <= 1} onClick={() => setPage((p) => p - 1)}>
            이전
          </button>
          <span>
            {page} / {totalPages}
          </span>
          <button disabled={page >= totalPages} onClick={() => setPage((p) => p + 1)}>
            다음
          </button>
        </div>
      )}
    </div>
  );
}
