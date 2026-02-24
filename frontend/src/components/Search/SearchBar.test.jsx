import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import SearchBar from './SearchBar';

describe('SearchBar', () => {
  const defaultProps = {
    onSearch: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('rendering', () => {
    it('renders input field', () => {
      render(<SearchBar {...defaultProps} />);
      expect(screen.getByRole('textbox')).toBeInTheDocument();
    });

    it('renders submit button', () => {
      render(<SearchBar {...defaultProps} />);
      expect(screen.getByRole('button', { name: /질문하기/i })).toBeInTheDocument();
    });

    it('renders search form with role=search', () => {
      render(<SearchBar {...defaultProps} />);
      expect(screen.getByRole('search')).toBeInTheDocument();
    });

    it('renders mode buttons when onModeChange is provided', () => {
      const onModeChange = jest.fn();
      render(<SearchBar {...defaultProps} onModeChange={onModeChange} />);
      expect(screen.getByRole('button', { name: /AI에게 질문/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /유사 문서 검색/i })).toBeInTheDocument();
    });

    it('does not render mode buttons when onModeChange is not provided', () => {
      render(<SearchBar {...defaultProps} />);
      expect(screen.queryByRole('button', { name: /AI에게 질문/i })).not.toBeInTheDocument();
    });
  });

  describe('placeholder', () => {
    it('shows ask mode placeholder by default', () => {
      render(<SearchBar {...defaultProps} mode="ask" />);
      expect(screen.getByPlaceholderText(/팀 지식에 대해 질문하세요/i)).toBeInTheDocument();
    });

    it('shows semantic mode placeholder when mode is semantic', () => {
      render(<SearchBar {...defaultProps} mode="semantic" />);
      expect(screen.getByPlaceholderText(/문서를 의미 기반으로 검색하세요/i)).toBeInTheDocument();
    });

    it('uses custom placeholder when provided', () => {
      render(<SearchBar {...defaultProps} placeholder="Custom placeholder" />);
      expect(screen.getByPlaceholderText('Custom placeholder')).toBeInTheDocument();
    });
  });

  describe('initial value', () => {
    it('sets initial value in input', () => {
      render(<SearchBar {...defaultProps} initialValue="initial query" />);
      expect(screen.getByRole('textbox')).toHaveValue('initial query');
    });
  });

  describe('search submission', () => {
    it('calls onSearch with trimmed query on submit', async () => {
      const onSearch = jest.fn();
      const user = userEvent.setup();
      render(<SearchBar onSearch={onSearch} />);

      await user.type(screen.getByRole('textbox'), '  test query  ');
      await user.click(screen.getByRole('button', { name: /질문하기/i }));

      expect(onSearch).toHaveBeenCalledWith('test query');
    });

    it('does not call onSearch when query is empty', async () => {
      const onSearch = jest.fn();
      const user = userEvent.setup();
      render(<SearchBar onSearch={onSearch} />);

      await user.click(screen.getByRole('button', { name: /질문하기/i }));

      expect(onSearch).not.toHaveBeenCalled();
    });

    it('does not call onSearch when query is only whitespace', async () => {
      const onSearch = jest.fn();
      const user = userEvent.setup();
      render(<SearchBar onSearch={onSearch} />);

      await user.type(screen.getByRole('textbox'), '   ');
      // Submit button should be disabled
      expect(screen.getByRole('button', { name: /질문하기/i })).toBeDisabled();
    });

    it('submits on Enter key press', async () => {
      const onSearch = jest.fn();
      const user = userEvent.setup();
      render(<SearchBar onSearch={onSearch} />);

      const input = screen.getByRole('textbox');
      await user.type(input, 'enter test{Enter}');

      expect(onSearch).toHaveBeenCalledWith('enter test');
    });
  });

  describe('clear button', () => {
    it('shows clear button when query is not empty', async () => {
      const user = userEvent.setup();
      render(<SearchBar {...defaultProps} />);

      await user.type(screen.getByRole('textbox'), 'test');

      expect(screen.getByRole('button', { name: /검색어 지우기/i })).toBeInTheDocument();
    });

    it('hides clear button when query is empty', () => {
      render(<SearchBar {...defaultProps} />);
      expect(screen.queryByRole('button', { name: /검색어 지우기/i })).not.toBeInTheDocument();
    });

    it('clears input when clear button is clicked', async () => {
      const user = userEvent.setup();
      render(<SearchBar {...defaultProps} />);

      await user.type(screen.getByRole('textbox'), 'test query');
      await user.click(screen.getByRole('button', { name: /검색어 지우기/i }));

      expect(screen.getByRole('textbox')).toHaveValue('');
    });
  });

  describe('mode change', () => {
    it('calls onModeChange with ask when ask button clicked', async () => {
      const onModeChange = jest.fn();
      const user = userEvent.setup();
      render(<SearchBar {...defaultProps} onModeChange={onModeChange} mode="semantic" />);

      await user.click(screen.getByRole('button', { name: /AI에게 질문/i }));

      expect(onModeChange).toHaveBeenCalledWith('ask');
    });

    it('calls onModeChange with semantic when semantic button clicked', async () => {
      const onModeChange = jest.fn();
      const user = userEvent.setup();
      render(<SearchBar {...defaultProps} onModeChange={onModeChange} mode="ask" />);

      await user.click(screen.getByRole('button', { name: /유사 문서 검색/i }));

      expect(onModeChange).toHaveBeenCalledWith('semantic');
    });

    it('shows correct button as active based on mode', () => {
      render(<SearchBar {...defaultProps} onModeChange={jest.fn()} mode="ask" />);

      const askButton = screen.getByRole('button', { name: /AI에게 질문/i });
      const semanticButton = screen.getByRole('button', { name: /유사 문서 검색/i });

      expect(askButton).toHaveAttribute('aria-pressed', 'true');
      expect(semanticButton).toHaveAttribute('aria-pressed', 'false');
    });
  });

  describe('loading state', () => {
    it('disables input when loading', () => {
      render(<SearchBar {...defaultProps} loading={true} />);
      expect(screen.getByRole('textbox')).toBeDisabled();
    });

    it('disables submit button when loading', () => {
      render(<SearchBar {...defaultProps} loading={true} initialValue="test" />);
      expect(screen.getByRole('button', { name: /질문하기/i })).toBeDisabled();
    });

    it('shows spinner in submit button when loading', () => {
      render(<SearchBar {...defaultProps} loading={true} initialValue="test" />);
      expect(document.querySelector('.kb-search-submit-spinner')).toBeInTheDocument();
    });
  });

  describe('submit button text', () => {
    it('shows "질문하기" in ask mode', () => {
      render(<SearchBar {...defaultProps} mode="ask" initialValue="test" />);
      expect(screen.getByRole('button', { name: /질문하기/i })).toBeInTheDocument();
    });

    it('shows "검색" in semantic mode', () => {
      render(<SearchBar {...defaultProps} mode="semantic" initialValue="test" />);
      expect(screen.getByRole('button', { name: /검색하기/i })).toBeInTheDocument();
    });
  });

  describe('accessibility', () => {
    it('has correct aria-label for input in ask mode', () => {
      render(<SearchBar {...defaultProps} mode="ask" />);
      expect(screen.getByRole('textbox')).toHaveAttribute('aria-label', 'AI 질문 입력');
    });

    it('has correct aria-label for input in semantic mode', () => {
      render(<SearchBar {...defaultProps} mode="semantic" />);
      expect(screen.getByRole('textbox')).toHaveAttribute('aria-label', '검색어 입력');
    });

    it('mode buttons group has correct aria-label', () => {
      render(<SearchBar {...defaultProps} onModeChange={jest.fn()} />);
      expect(screen.getByRole('group')).toHaveAttribute('aria-label', '검색 모드');
    });
  });
});
