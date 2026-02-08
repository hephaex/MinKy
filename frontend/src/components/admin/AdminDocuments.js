import React from 'react';

const DocumentCard = ({ doc }) => (
  <div className="document-card">
    <div className="document-header">
      <h3>{doc.title}</h3>
      <span className={`visibility-badge ${doc.is_public ? 'public' : 'private'}`}>
        {doc.is_public ? 'Public' : 'Private'}
      </span>
    </div>
    <div className="document-meta">
      <p>Author: {doc.owner?.username || 'Unknown'}</p>
      <p>Created: {new Date(doc.created_at).toLocaleDateString()}</p>
      <p>Updated: {new Date(doc.updated_at).toLocaleDateString()}</p>
      <p>Tags: {doc.tags?.map(tag => tag.name).join(', ') || 'None'}</p>
    </div>
  </div>
);

const AdminDocuments = ({ documents, currentPage, onPageChange }) => {
  return (
    <div className="documents-tab">
      <h2>Document Management</h2>

      <div className="documents-list">
        {documents.map(doc => (
          <DocumentCard key={doc.id} doc={doc} />
        ))}
      </div>

      <div className="pagination">
        <button
          disabled={currentPage === 1}
          onClick={() => onPageChange(currentPage - 1)}
        >
          Previous
        </button>
        <span>Page {currentPage}</span>
        <button onClick={() => onPageChange(currentPage + 1)}>
          Next
        </button>
      </div>
    </div>
  );
};

export default AdminDocuments;
