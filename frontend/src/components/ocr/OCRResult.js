import PropTypes from 'prop-types';

const OCRResult = ({ result, mode }) => {
  const getText = () => result.ocr_result?.text || result.text;
  const getMethod = () => result.ocr_result?.method || result.method;
  const getConfidence = () => result.ocr_result?.confidence || result.confidence;
  const getWordCount = () => result.ocr_result?.word_count || result.word_count;

  const handleCopyToClipboard = () => {
    navigator.clipboard.writeText(getText());
  };

  return (
    <div className="ocr-result">
      <div className="result-header">
        <h4>{mode === 'create' ? 'Document Created' : 'Text Extracted'} Successfully</h4>
        <div className="result-meta">
          <span className="meta-item">
            Method: <strong>{getMethod()}</strong>
          </span>
          <span className="meta-item">
            Confidence: <strong>{getConfidence()}%</strong>
          </span>
          <span className="meta-item">
            Words: <strong>{getWordCount()}</strong>
          </span>
        </div>
      </div>

      {mode === 'create' && result.document && (
        <div className="document-info">
          <p>
            <strong>Document created:</strong>{' '}
            <a href={`/documents/${result.document.id}`} target="_blank" rel="noopener noreferrer">
              {result.document.title}
            </a>
          </p>
        </div>
      )}

      <div className="extracted-text">
        <div className="text-header">
          <strong>Extracted Text:</strong>
          <button
            onClick={handleCopyToClipboard}
            className="btn btn-sm btn-secondary"
            title="Copy to clipboard"
          >
            📋 Copy
          </button>
        </div>
        <pre className="text-content">{getText()}</pre>
      </div>
    </div>
  );
};

OCRResult.propTypes = {
  result: PropTypes.shape({
    text: PropTypes.string,
    method: PropTypes.string,
    confidence: PropTypes.number,
    word_count: PropTypes.number,
    ocr_result: PropTypes.shape({
      text: PropTypes.string,
      method: PropTypes.string,
      confidence: PropTypes.number,
      word_count: PropTypes.number,
    }),
    document: PropTypes.shape({
      id: PropTypes.oneOfType([PropTypes.string, PropTypes.number]),
      title: PropTypes.string,
    }),
  }).isRequired,
  mode: PropTypes.oneOf(['create', 'extract']).isRequired,
};

export default OCRResult;
