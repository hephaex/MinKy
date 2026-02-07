import React from 'react';

const DocumentClusters = ({ clusteringResults, clusteringConfig, setClusteringConfig, onCluster }) => {
  const handleConfigChange = (key, value) => {
    setClusteringConfig(prev => ({
      ...prev,
      [key]: value
    }));
  };

  return (
    <div className="document-clusters">
      <div className="section-header">
        <h4>Document Clustering</h4>
      </div>

      <div className="clustering-config">
        <div className="config-row">
          <div className="config-group">
            <label>Clustering Method:</label>
            <select
              value={clusteringConfig.method}
              onChange={(e) => handleConfigChange('method', e.target.value)}
              className="form-control"
            >
              <option value="auto">Auto-select</option>
              <option value="kmeans">K-Means</option>
              <option value="hierarchical">Hierarchical</option>
              <option value="dbscan">DBSCAN</option>
            </select>
          </div>

          <div className="config-group">
            <label>Number of Clusters:</label>
            <input
              type="number"
              value={clusteringConfig.n_clusters || ''}
              onChange={(e) => handleConfigChange('n_clusters', e.target.value ? parseInt(e.target.value) : null)}
              placeholder="Auto-detect"
              min="2"
              max="20"
              className="form-control"
            />
          </div>

          <div className="config-group">
            <label>Scope:</label>
            <select
              value={clusteringConfig.scope}
              onChange={(e) => handleConfigChange('scope', e.target.value)}
              className="form-control"
            >
              <option value="user">My Documents</option>
              <option value="public">Public Documents</option>
            </select>
          </div>

          <div className="config-group">
            <label>Max Documents:</label>
            <input
              type="number"
              value={clusteringConfig.max_documents}
              onChange={(e) => handleConfigChange('max_documents', parseInt(e.target.value))}
              min="10"
              max="1000"
              className="form-control"
            />
          </div>
        </div>

        <button onClick={onCluster} className="btn btn-primary">
          Cluster Documents
        </button>
      </div>

      {clusteringResults && (
        <div className="clustering-results">
          <div className="cluster-overview">
            <h5>Clustering Results</h5>
            <div className="overview-stats">
              <div className="overview-item">
                <span className="overview-label">Method Used:</span>
                <span className="overview-value">{clusteringResults.method}</span>
              </div>
              <div className="overview-item">
                <span className="overview-label">Clusters Found:</span>
                <span className="overview-value">{clusteringResults.n_clusters}</span>
              </div>
              <div className="overview-item">
                <span className="overview-label">Documents Processed:</span>
                <span className="overview-value">{clusteringResults.documents_processed}</span>
              </div>
              {clusteringResults.quality_metrics && clusteringResults.quality_metrics.silhouette_score && (
                <div className="overview-item">
                  <span className="overview-label">Quality Score:</span>
                  <span className="overview-value">
                    {clusteringResults.quality_metrics.silhouette_score.toFixed(3)}
                  </span>
                </div>
              )}
            </div>
          </div>

          <div className="clusters-display">
            {Object.entries(clusteringResults.clusters).map(([clusterId, docs]) => (
              <div key={clusterId} className="cluster-card">
                <div className="cluster-header">
                  <h6>Cluster {clusterId}</h6>
                  <span className="cluster-size">{docs.length} documents</span>
                </div>

                {clusteringResults.cluster_insights && clusteringResults.cluster_insights[clusterId] && (
                  <div className="cluster-insights">
                    <div className="cluster-topics">
                      <span className="topics-label">Key terms:</span>
                      <div className="topics-list">
                        {clusteringResults.cluster_insights[clusterId].top_terms.map((term, i) => (
                          <span key={i} className="topic-tag">{term}</span>
                        ))}
                      </div>
                    </div>

                    {clusteringResults.cluster_insights[clusterId].common_tags.length > 0 && (
                      <div className="cluster-tags">
                        <span className="tags-label">Common tags:</span>
                        <div className="tags-list">
                          {clusteringResults.cluster_insights[clusterId].common_tags.map((tag, i) => (
                            <span key={i} className="tag-badge">{tag}</span>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}

                <div className="cluster-documents">
                  {docs.slice(0, 5).map((doc) => (
                    <div key={doc.id} className="cluster-doc">
                      <a href={`/documents/${doc.id}`} className="doc-link">
                        {doc.title}
                      </a>
                      <span className="doc-author">by {doc.author}</span>
                    </div>
                  ))}
                  {docs.length > 5 && (
                    <div className="more-docs">
                      +{docs.length - 5} more documents
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default DocumentClusters;
