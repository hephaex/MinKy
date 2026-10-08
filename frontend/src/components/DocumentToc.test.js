import { render, screen, fireEvent, waitFor } from '../test-utils';
import DocumentToc, { TOC_MODES } from './DocumentToc';
import { documentService } from '../services/api';

jest.mock('../services/api', () => ({
  __esModule: true,
  default: { get: jest.fn(), post: jest.fn() },
  documentService: {
    getDocuments: jest.fn(),
  },
}));

describe('DocumentToc', () => {
  const headings = [
    { level: 1, text: '개요', slug: '개요' },
    { level: 2, text: '설치 방법', slug: '설치-방법' },
    { level: 2, text: '사용법', slug: '사용법' },
  ];

  beforeEach(() => {
    documentService.getDocuments.mockReset();
    documentService.getDocuments.mockResolvedValue({ documents: [] });
  });

  it('defaults to the TOC tab and lists every heading', () => {
    render(
      <DocumentToc
        headings={headings}
        currentDocumentId="1"
        mode={TOC_MODES.TOC}
        onModeChange={jest.fn()}
      />
    );

    expect(screen.getByRole('tab', { name: '목차' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tab', { name: '최근 보기' })).toHaveAttribute(
      'aria-selected',
      'false'
    );
    expect(screen.getByText('개요')).toBeInTheDocument();
    expect(screen.getByText('설치 방법')).toBeInTheDocument();
    expect(screen.getByText('사용법')).toBeInTheDocument();
    // Recent mode fetch must not fire while the TOC tab is active.
    expect(documentService.getDocuments).not.toHaveBeenCalled();
  });

  it('shows an empty state when the document has no headings', () => {
    render(
      <DocumentToc
        headings={[]}
        currentDocumentId="1"
        mode={TOC_MODES.TOC}
        onModeChange={jest.fn()}
      />
    );

    expect(screen.getByText('이 문서에는 제목이 없습니다')).toBeInTheDocument();
  });

  it('clicking a TOC entry smooth-scrolls to the matching heading id', () => {
    const target = window.document.createElement('div');
    target.id = '설치-방법';
    target.scrollIntoView = jest.fn();
    target.focus = jest.fn();
    window.document.body.appendChild(target);

    render(
      <DocumentToc
        headings={headings}
        currentDocumentId="1"
        mode={TOC_MODES.TOC}
        onModeChange={jest.fn()}
      />
    );

    fireEvent.click(screen.getByText('설치 방법'));

    expect(target.scrollIntoView).toHaveBeenCalledWith({ behavior: 'smooth', block: 'start' });
    expect(target.focus).toHaveBeenCalled();

    window.document.body.removeChild(target);
  });

  it('calls onModeChange when the 최근 보기 tab is clicked', () => {
    const onModeChange = jest.fn();
    render(
      <DocumentToc
        headings={headings}
        currentDocumentId="1"
        mode={TOC_MODES.TOC}
        onModeChange={onModeChange}
      />
    );

    fireEvent.click(screen.getByRole('tab', { name: '최근 보기' }));
    expect(onModeChange).toHaveBeenCalledWith(TOC_MODES.RECENT);
  });

  it('fetches and renders recent documents, excluding the currently open one', async () => {
    documentService.getDocuments.mockResolvedValue({
      documents: [
        { id: 1, title: '현재 문서', created_at: '2026-07-01T00:00:00Z' },
        { id: 2, title: '다른 문서', created_at: '2026-07-02T00:00:00Z' },
      ],
    });

    render(
      <DocumentToc
        headings={headings}
        currentDocumentId="1"
        mode={TOC_MODES.RECENT}
        onModeChange={jest.fn()}
      />
    );

    await waitFor(() => expect(documentService.getDocuments).toHaveBeenCalledWith(1, 5, ''));
    await waitFor(() => expect(screen.getByText('다른 문서')).toBeInTheDocument());
    expect(screen.queryByText('현재 문서')).not.toBeInTheDocument();
  });

  it('shows an error state when the recent-documents fetch fails', async () => {
    documentService.getDocuments.mockRejectedValue(new Error('network'));

    render(
      <DocumentToc
        headings={headings}
        currentDocumentId="1"
        mode={TOC_MODES.RECENT}
        onModeChange={jest.fn()}
      />
    );

    await waitFor(() =>
      expect(screen.getByText('최근 문서를 불러올 수 없습니다')).toBeInTheDocument()
    );
  });
});
