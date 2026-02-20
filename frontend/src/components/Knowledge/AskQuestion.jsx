import React, { useState, useCallback } from 'react';
import PropTypes from 'prop-types';
import { SearchBar } from '../Search';
import AnswerDisplay from './AnswerDisplay';
import { searchService } from '../../services/api';
import './AskQuestion.css';

const EXAMPLE_QUESTIONS = [
  '우리 팀이 Redis 캐싱 문제를 어떻게 해결했나요?',
  '인증 시스템은 어떻게 구현되어 있나요?',
  '최근에 배포한 주요 기능이 무엇인가요?',
  '코드 리뷰 프로세스는 어떻게 되나요?',
];

const AskQuestion = ({ onSearch }) => {
  const [state, setState] = useState({
    question: '',
    answer: null,
    sources: [],
    loading: false,
    error: null,
  });

  const handleAsk = useCallback(async (question) => {
    setState((prev) => ({
      ...prev,
      question,
      answer: null,
      sources: [],
      loading: true,
      error: null,
    }));

    try {
      const result = await searchService.ask(question);
      setState((prev) => ({
        ...prev,
        answer: result.answer || result.data?.answer || '',
        sources: result.sources || result.data?.sources || [],
        loading: false,
      }));

      if (onSearch) {
        onSearch({ question, result });
      }
    } catch (err) {
      setState((prev) => ({
        ...prev,
        loading: false,
        error: err.response?.data?.error || err.message || '질문 처리 중 오류가 발생했습니다.',
      }));
    }
  }, [onSearch]);

  const handleRetry = useCallback(() => {
    if (state.question) {
      handleAsk(state.question);
    }
  }, [state.question, handleAsk]);

  const handleExampleClick = useCallback((question) => {
    handleAsk(question);
  }, [handleAsk]);

  return (
    <div className="kb-ask">
      <div className="kb-ask-search">
        <SearchBar
          onSearch={handleAsk}
          mode="ask"
          loading={state.loading}
        />
      </div>

      {!state.question && !state.loading && (
        <div className="kb-ask-examples" aria-label="질문 예시">
          <p className="kb-ask-examples-label">예시 질문</p>
          <ul className="kb-ask-examples-list">
            {EXAMPLE_QUESTIONS.map((q, i) => (
              <li key={i}>
                <button
                  type="button"
                  className="kb-ask-example-btn"
                  onClick={() => handleExampleClick(q)}
                >
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" aria-hidden="true">
                    <polyline points="9 18 15 12 9 6" />
                  </svg>
                  {q}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}

      {(state.loading || state.answer || state.error) && (
        <div className="kb-ask-result">
          {state.error ? (
            <div className="kb-ask-error" role="alert">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#dc3545" strokeWidth="2" aria-hidden="true">
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="8" x2="12" y2="12" />
                <line x1="12" y1="16" x2="12.01" y2="16" />
              </svg>
              <span>{state.error}</span>
              <button type="button" className="kb-ask-retry" onClick={handleRetry}>
                다시 시도
              </button>
            </div>
          ) : (
            <AnswerDisplay
              answer={state.answer}
              sources={state.sources}
              question={state.question}
              streaming={state.loading}
            />
          )}
        </div>
      )}
    </div>
  );
};

AskQuestion.propTypes = {
  onSearch: PropTypes.func,
};

export default AskQuestion;
