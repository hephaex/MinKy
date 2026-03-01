import PropTypes from 'prop-types';

const DocumentClusters = ({
  clusteringResults,
  clusteringConfig,
  setClusteringConfig,
  onCluster,
}) => {
  const handleConfigChange = (key, value) => {
    setClusteringConfig((prev) => ({
      ...prev,
      [key]: value,
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
            <label htmlFor="clustering-method">Clustering Method:</label>
            <select
              id="clustering-method"
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
            <label htmlFor="n-clusters">Number of Clusters:</label>
            <input
              id="n-clusters"
              type="number"
              value={clusteringConfig.n_clusters || ''}
              onChange={(e) =>
                handleConfigChange('n_clusters', e.target.value ? parseInt(e.target.value) : null)
              }
              placeholder="Auto-detect"
              min="2"
              max="20"
              className="form-control"
            />
          </div>

          <div className="config-group">
            <label htmlFor="clustering-scope">Scope:</label>
            <select
              id="clustering-scope"
              value={clusteringConfig.scope}
              onChange={(e) => handleConfigChange('scope', e.target.value)}
              className="form-control"
            >
              <option value="user">My Documents</option>
              <option value="public">Public Documents</option>
            </select>
          </div>

          <div className="config-group">
            <label htmlFor="max-documents">Max Documents:</label>
            <input
              id="max-documents"
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
              {clusteringResults.quality_metrics &&
                clusteringResults.quality_metrics.silhouette_score && (
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

                {clusteringResults.cluster_insights &&
                  clusteringResults.cluster_insights[clusterId] && (
                    <div className="cluster-insights">
                      <div className="cluster-topics">
                        <span className="topics-label">Key terms:</span>
                        <div className="topics-list">
                          {clusteringResults.cluster_insights[clusterId].top_terms.map(
                            (term, i) => (
                              <span key={i} className="topic-tag">
                                {term}
                              </span>
                            )
                          )}
                        </div>
                      </div>

                      {clusteringResults.cluster_insights[clusterId].common_tags.length > 0 && (
                        <div className="cluster-tags">
                          <span className="tags-label">Common tags:</span>
                          <div className="tags-list">
                            {clusteringResults.cluster_insights[clusterId].common_tags.map(
                              (tag, i) => (
                                <span key={i} className="tag-badge">
                                  {tag}
                                </span>
                              )
                            )}
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
                    <div className="more-docs">+{docs.length - 5} more documents</div>
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

const clusterDocShape = {
  id: PropTypes.oneOfType([PropTypes.string, PropTypes.number]).isRequired,
  title: PropTypes.string.isRequired,
  author: PropTypes.string,
};

DocumentClusters.propTypes = {
  clusteringResults: PropTypes.shape({
    method: PropTypes.string,
    n_clusters: PropTypes.number,
    documents_processed: PropTypes.number,
    quality_metrics: PropTypes.shape({
      silhouette_score: PropTypes.number,
    }),
    clusters: PropTypes.objectOf(PropTypes.arrayOf(PropTypes.shape(clusterDocShape))),
    cluster_insights: PropTypes.objectOf(
      PropTypes.shape({
        top_terms: PropTypes.arrayOf(PropTypes.string),
        common_tags: PropTypes.arrayOf(PropTypes.string),
      })
    ),
  }),
  clusteringConfig: PropTypes.shape({
    method: PropTypes.string,
    n_clusters: PropTypes.number,
    scope: PropTypes.string,
    max_documents: PropTypes.number,
  }).isRequired,
  setClusteringConfig: PropTypes.func.isRequired,
  onCluster: PropTypes.func.isRequired,
};

DocumentClusters.defaultProps = {
  clusteringResults: null,
};

export default DocumentClusters;
