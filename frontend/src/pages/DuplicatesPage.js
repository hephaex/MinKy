import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { archiveService } from '../services/api';
import './DuplicatesPage.css';

export default function DuplicatesPage() {
  const [groups, setGroups] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [actionMsg, setActionMsg] = useState(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await archiveService.getDuplicates();
      setGroups(data.groups || []);
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

  const handleRestore = async (id, title) => {
    try {
      await archiveService.restoreDocument(id);
      setActionMsg(`"${title}" 복원 완료`);
      await load();
    } catch (e) {
      setActionMsg(`오류: ${e.response?.data?.error || e.message}`);
    }
  };

  return (
    <div className="duplicates-page">
      <div className="page-header">
        <h1>중복 문서 관리</h1>
        <Link to="/archived" className="archived-link">
          아카이브 목록 →
        </Link>
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
              <li key={doc.id} className={`doc-item ${doc.archived ? 'archived' : ''}`}>
                <div className="doc-info">
                  <Link to={`/documents/${doc.id}`} className="doc-title">
                    {doc.title}
                  </Link>
                  <span className="doc-date">
                    {new Date(doc.created_at).toLocaleDateString('ko-KR')}
                  </span>
                  {doc.archived && <span className="badge-archived">아카이브됨</span>}
                </div>
                <div className="doc-actions">
                  {!doc.archived ? (
                    <button
                      className="btn-archive"
                      onClick={() => handleArchive(doc.id, doc.title)}
                    >
                      아카이브
                    </button>
                  ) : (
                    <button
                      className="btn-restore"
                      onClick={() => handleRestore(doc.id, doc.title)}
                    >
                      복원
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}
