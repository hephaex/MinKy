// 옵시디언 스타일 마크다운 렌더링 유틸리티
import React from 'react';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { tomorrow } from 'react-syntax-highlighter/dist/esm/styles/prism';

export const processInternalLinks = (content, navigate, documentLookup = {}) => {
  // [[link|display]] 또는 [[link]] 패턴 처리
  const linkPattern = /\[\[([^\|\]]+)(?:\|([^\]]+))?\]\]/g;
  
  return content.replace(linkPattern, (match, target, displayText) => {
    const display = displayText || target;
    const docId = documentLookup[target];
    
    if (docId) {
      return `<a href="/documents/${docId}" class="internal-link" data-target="${target}">${display}</a>`;
    } else {
      return `<span class="internal-link broken" data-target="${target}" title="문서를 찾을 수 없습니다">${display}</span>`;
    }
  });
};

export const processHashtags = (content) => {
  // #tag 패턴 처리 (워드 경계 고려)
  const hashtagPattern = /(?:^|\s)(#([a-zA-Z가-힣][a-zA-Z0-9가-힣_-]*))/g;
  
  return content.replace(hashtagPattern, (match, fullTag, tagName) => {
    const prefix = match.substring(0, match.indexOf('#'));
    return `${prefix}<a href="/tags/${tagName}" class="hashtag">${fullTag}</a>`;
  });
};

export const extractFrontmatter = (content) => {
  const frontmatterPattern = /^---\s*\n(.*?)\n---\s*\n/s;
  const match = content.match(frontmatterPattern);
  
  if (match) {
    try {
      // 간단한 YAML 파싱 (프론트엔드용)
      const yamlContent = match[1];
      const metadata = {};
      
      yamlContent.split('\n').forEach(line => {
        const colonIndex = line.indexOf(':');
        if (colonIndex > 0) {
          const key = line.substring(0, colonIndex).trim();
          let value = line.substring(colonIndex + 1).trim();
          
          // 따옴표 제거
          if ((value.startsWith('"') && value.endsWith('"')) || 
              (value.startsWith("'") && value.endsWith("'"))) {
            value = value.slice(1, -1);
          }
          
          // 배열 처리 (간단한 형태)
          if (value.startsWith('[') && value.endsWith(']')) {
            value = value.slice(1, -1).split(',').map(v => v.trim().replace(/['"]/g, ''));
          }
          
          metadata[key] = value;
        }
      });
      
      return {
        metadata,
        content: content.substring(match[0].length)
      };
    } catch (error) {
      console.warn('프론트매터 파싱 실패:', error);
    }
  }
  
  return {
    metadata: {},
    content
  };
};

export const createCustomMarkdownComponents = (navigate, documentLookup = {}) => {
  return {
    // 텍스트 노드에서 내부 링크와 해시태그 처리
    text({ children }) {
      if (typeof children !== 'string') return children;
      
      // 내부 링크 처리
      let processed = processInternalLinks(children, navigate, documentLookup);
      
      // 해시태그 처리
      processed = processHashtags(processed);
      
      // HTML이 포함되어 있으면 dangerouslySetInnerHTML 사용
      if (processed !== children && (processed.includes('<a') || processed.includes('<span'))) {
        return <span dangerouslySetInnerHTML={{ __html: processed }} />;
      }
      
      return children;
    },
    
    // 코드 블록 처리 (기존 로직 유지)
    code({ node, inline, className, children, ...props }) {
      const match = /language-(\w+)/.exec(className || '');
      return !inline && match ? (
        <SyntaxHighlighter
          style={tomorrow}
          language={match[1]}
          PreTag="div"
          {...props}
        >
          {String(children).replace(/\n$/, '')}
        </SyntaxHighlighter>
      ) : (
        <code className={className} {...props}>
          {children}
        </code>
      );
    }
  };
};