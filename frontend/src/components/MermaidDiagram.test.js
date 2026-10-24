import { render, screen, waitFor } from '../test-utils';

jest.mock('mermaid', () => ({
  __esModule: true,
  default: {
    initialize: jest.fn(),
    render: jest.fn(),
  },
}));

jest.mock('../utils/logger', () => ({
  logError: jest.fn(),
}));

import { logError } from '../utils/logger';
import mermaid from 'mermaid';

const SAMPLE = 'flowchart TB\n  A --> B';

const renderDiagram = (code = SAMPLE) => render(<MermaidDiagram code={code} />);

import MermaidDiagram from './MermaidDiagram';

beforeEach(() => {
  jest.clearAllMocks();
  logError.mockClear();
});

describe('MermaidDiagram', () => {
  test('renders svg after successful mermaid.render', async () => {
    mermaid.render.mockResolvedValue({ svg: '<svg data-testid="rendered-svg"><g/></svg>' });
    renderDiagram();
    await waitFor(() => {
      expect(screen.getByTestId('rendered-svg')).toBeInTheDocument();
    });
    const container = screen.getByTestId('rendered-svg').closest('.mermaid-diagram');
    expect(container).toBeInTheDocument();
    expect(container).toHaveAttribute('role', 'img');
  });

  test('initializes mermaid once with strict security and startOnLoad false', async () => {
    mermaid.render.mockResolvedValue({ svg: '<svg/>' });
    renderDiagram();
    await waitFor(() => {
      expect(mermaid.initialize).toHaveBeenCalled();
    });
    expect(mermaid.initialize).toHaveBeenCalledWith(
      expect.objectContaining({ startOnLoad: false, securityLevel: 'strict' })
    );
  });

  test('passes the sanitized alphanumeric render id and code to mermaid.render', async () => {
    mermaid.render.mockResolvedValue({ svg: '<svg/>' });
    renderDiagram();
    await waitFor(() => {
      expect(mermaid.render).toHaveBeenCalled();
    });
    const [renderId, code] = mermaid.render.mock.calls[0];
    expect(renderId).toMatch(/^mermaid-[a-zA-Z0-9]+$/);
    expect(code).toBe(SAMPLE);
  });

  test('falls back to raw code block on render failure', async () => {
    mermaid.render.mockRejectedValue(new Error('Parse error'));
    renderDiagram();
    await waitFor(() => {
      expect(screen.getByText(/flowchart TB/)).toBeInTheDocument();
    });
    expect(screen.getByText(/flowchart TB/).closest('pre')).toBeInTheDocument();
    expect(logError).toHaveBeenCalled();
  });

  test('shows loading placeholder before render resolves', () => {
    let resolveRender;
    mermaid.render.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveRender = resolve;
        })
    );
    renderDiagram();
    expect(screen.getByText('다이어그램 렌더링 중…')).toBeInTheDocument();
    resolveRender?.({ svg: '<svg/>' });
  });
});
