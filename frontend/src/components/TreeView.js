import { useState, useCallback, useRef, useEffect } from 'react';
import PropTypes from 'prop-types';

const TreeNode = ({
  node,
  level,
  expandedNodes,
  onToggle,
  onSelect,
  onNodeClick,
  focusedId = null,
  onFocusChange,
}) => {
  const nodeRef = useRef(null);
  const hasChildren = node.children && node.children.length > 0;
  const isExpanded = expandedNodes.has(node.id);
  const isFocused = focusedId === node.id;
  const isDocument = node.type === 'document';

  useEffect(() => {
    if (isFocused && nodeRef.current) {
      nodeRef.current.focus();
    }
  }, [isFocused]);

  // Arrow click: only toggle expand/collapse, do not navigate.
  const handleArrowClick = (e) => {
    e.stopPropagation();
    if (hasChildren) {
      onToggle(node.id);
    }
  };

  // Row click: expand/collapse folder via onToggle, or select document via onSelect.
  // Falls back to onNodeClick for folders without children (custom navigation).
  const handleClick = () => {
    if (isDocument) {
      onSelect(node);
    } else if (hasChildren) {
      onToggle(node.id);
    } else if (onNodeClick) {
      onNodeClick(node);
    }
  };

  const handleKeyDown = (e) => {
    switch (e.key) {
      case 'Enter':
      case ' ':
        e.preventDefault();
        handleClick();
        break;
      case 'ArrowRight':
        e.preventDefault();
        if (hasChildren && !isExpanded) {
          onToggle(node.id);
        } else if (onFocusChange) {
          onFocusChange(node.id);
        }
        break;
      case 'ArrowLeft':
        e.preventDefault();
        if (hasChildren && isExpanded) {
          onToggle(node.id);
        }
        break;
      default:
        break;
    }
  };

  return (
    <div className="tree-node-container">
      <div
        ref={nodeRef}
        className={`tree-node ${isDocument ? 'tree-node-leaf' : 'tree-node-branch'} ${isFocused ? 'tree-node-focused' : ''}`}
        style={{ paddingLeft: `${level * 16 + 8}px` }}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
        role="treeitem"
        tabIndex={isFocused ? 0 : -1}
        aria-expanded={hasChildren ? isExpanded : undefined}
        aria-selected={isDocument && isFocused}
      >
        {hasChildren && (
          <span
            className={`tree-expand-icon ${isExpanded ? 'expanded' : ''}`}
            onClick={handleArrowClick}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                handleArrowClick(e);
              }
            }}
            role="button"
            tabIndex={-1}
            aria-label={isExpanded ? '접기' : '펼치기'}
          >
            <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
              <path d="M3 1l5 4-5 4z" />
            </svg>
          </span>
        )}
        {!hasChildren && <span className="tree-expand-spacer" />}

        {node.color && !isDocument && (
          <span className="tree-node-color" style={{ backgroundColor: node.color }} />
        )}

        <span className="tree-node-label" title={node.label}>
          {isDocument ? '📄 ' : ''}
          {node.label}
        </span>

        {!isDocument && node.count !== undefined && (
          <span className="tree-node-count">{node.count}</span>
        )}
      </div>

      {hasChildren && isExpanded && (
        <div className="tree-children" role="group">
          {node.children.map((child) => (
            <TreeNode
              key={child.id}
              node={child}
              level={level + 1}
              expandedNodes={expandedNodes}
              onToggle={onToggle}
              onSelect={onSelect}
              onNodeClick={onNodeClick}
              focusedId={focusedId}
              onFocusChange={onFocusChange}
            />
          ))}
        </div>
      )}
    </div>
  );
};

const nodeShape = {
  id: PropTypes.string.isRequired,
  label: PropTypes.string.isRequired,
  // Matches the node_type vocabulary emitted by the Rust tree API
  type: PropTypes.oneOf(['document', 'folder', 'tag', 'year', 'month']),
  color: PropTypes.string,
  count: PropTypes.number,
};
// Allow recursive children
nodeShape.children = PropTypes.arrayOf(PropTypes.shape(nodeShape));

TreeNode.propTypes = {
  node: PropTypes.shape(nodeShape).isRequired,
  level: PropTypes.number.isRequired,
  expandedNodes: PropTypes.instanceOf(Set).isRequired,
  onToggle: PropTypes.func.isRequired,
  onSelect: PropTypes.func.isRequired,
  onNodeClick: PropTypes.func,
  focusedId: PropTypes.string,
  onFocusChange: PropTypes.func.isRequired,
};

const TreeView = ({ nodes = [], onSelect, onNodeClick, className = '' }) => {
  const [expandedNodes, setExpandedNodes] = useState(new Set());
  const [focusedId, setFocusedId] = useState(null);
  const treeRef = useRef(null);

  const getAllVisibleIds = useCallback(() => {
    const ids = [];
    const traverse = (nodeList) => {
      for (const node of nodeList) {
        ids.push(node.id);
        if (node.children && node.children.length > 0 && expandedNodes.has(node.id)) {
          traverse(node.children);
        }
      }
    };
    traverse(nodes);
    return ids;
  }, [nodes, expandedNodes]);

  const handleToggle = useCallback((nodeId) => {
    setExpandedNodes((prev) => {
      const next = new Set(prev);
      if (next.has(nodeId)) {
        next.delete(nodeId);
      } else {
        next.add(nodeId);
      }
      return next;
    });
  }, []);

  const handleTreeKeyDown = useCallback(
    (e) => {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const ids = getAllVisibleIds();
        if (ids.length === 0) return;

        const currentIndex = focusedId ? ids.indexOf(focusedId) : -1;
        let nextIndex;

        if (e.key === 'ArrowDown') {
          nextIndex = currentIndex < ids.length - 1 ? currentIndex + 1 : 0;
        } else {
          nextIndex = currentIndex > 0 ? currentIndex - 1 : ids.length - 1;
        }

        setFocusedId(ids[nextIndex]);
      }
    },
    [focusedId, getAllVisibleIds]
  );

  if (!nodes || nodes.length === 0) {
    return <div className="tree-empty">문서가 없습니다</div>;
  }

  return (
    <div
      ref={treeRef}
      className={`tree-view ${className || ''}`}
      role="tree"
      tabIndex={0}
      onKeyDown={handleTreeKeyDown}
    >
      {nodes.map((node) => (
        <TreeNode
          key={node.id}
          node={node}
          level={0}
          expandedNodes={expandedNodes}
          onToggle={handleToggle}
          onSelect={onSelect}
          onNodeClick={onNodeClick}
          focusedId={focusedId}
          onFocusChange={setFocusedId}
        />
      ))}
    </div>
  );
};

TreeView.propTypes = {
  nodes: PropTypes.arrayOf(PropTypes.shape(nodeShape)),
  onSelect: PropTypes.func.isRequired,
  onNodeClick: PropTypes.func,
  className: PropTypes.string,
};

export default TreeView;
