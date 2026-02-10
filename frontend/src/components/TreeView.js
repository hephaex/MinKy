import React, { useState, useCallback, useRef, useEffect } from 'react';

const TreeNode = ({ node, level, expandedNodes, onToggle, onSelect, focusedId, onFocusChange }) => {
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

  const handleClick = () => {
    if (isDocument) {
      onSelect(node);
    } else if (hasChildren) {
      onToggle(node.id);
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
        } else if (hasChildren && isExpanded && node.children.length > 0) {
          onFocusChange(node.children[0].id);
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
          <span className={`tree-expand-icon ${isExpanded ? 'expanded' : ''}`}>
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
          {isDocument ? '📄 ' : ''}{node.label}
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
              focusedId={focusedId}
              onFocusChange={onFocusChange}
            />
          ))}
        </div>
      )}
    </div>
  );
};

const TreeView = ({ nodes, onSelect, className }) => {
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

  const handleTreeKeyDown = useCallback((e) => {
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
  }, [focusedId, getAllVisibleIds]);

  if (!nodes || nodes.length === 0) {
    return (
      <div className="tree-empty">
        문서가 없습니다
      </div>
    );
  }

  return (
    <div
      ref={treeRef}
      className={`tree-view ${className || ''}`}
      role="tree"
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
          focusedId={focusedId}
          onFocusChange={setFocusedId}
        />
      ))}
    </div>
  );
};

export default TreeView;
