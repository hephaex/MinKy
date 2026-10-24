import { useEffect, useId, useState } from 'react';
import { logError } from '../utils/logger';
import PropTypes from 'prop-types';

/**
 * Renders a mermaid diagram definition as an inline SVG.
 *
 * mermaid v11 is imported dynamically so the ~1 MB renderer is code-split out
 * of the main bundle and only fetched when a document actually contains a
 * ```mermaid fence. Rendering failures (invalid syntax, unknown directive)
 * degrade to the raw definition inside the fallback block instead of throwing.
 */
const MermaidDiagram = ({ code }) => {
  const reactId = useId();
  // react useId produces ":r0:" style ids containing ':' which mermaid
  // rejects as an SVG id; normalize to alphanumerics.
  const renderId = `mermaid-${reactId.replace(/[^a-zA-Z0-9]/g, '')}`;
  const [svg, setSvg] = useState(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;

    const render = async () => {
      try {
        const mermaid = (await import('mermaid')).default;
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: 'strict',
          theme: 'default',
        });
        const { svg: rendered } = await mermaid.render(renderId, code);
        if (!cancelled) {
          setSvg(rendered);
          setFailed(false);
        }
      } catch (e) {
        logError(e, 'MermaidDiagram');
        if (!cancelled) {
          setSvg(null);
          setFailed(true);
        }
      }
    };

    render();
    return () => {
      cancelled = true;
    };
  }, [code, renderId]);

  if (failed) {
    return (
      <pre className="mermaid-fallback">
        <code>{code}</code>
      </pre>
    );
  }

  if (svg === null) {
    return (
      <div className="mermaid-loading" aria-busy="true">
        다이어그램 렌더링 중…
      </div>
    );
  }

  return <div className="mermaid-diagram" role="img" dangerouslySetInnerHTML={{ __html: svg }} />;
};
MermaidDiagram.propTypes = {
  code: PropTypes.string.isRequired,
};

export default MermaidDiagram;
