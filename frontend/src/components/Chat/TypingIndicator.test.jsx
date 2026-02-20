import React from 'react';
import { render } from '@testing-library/react';
import TypingIndicator from './TypingIndicator';

describe('TypingIndicator', () => {
  it('renders three dots', () => {
    const { container } = render(<TypingIndicator />);
    const dots = container.querySelectorAll('.chat-typing__dot');
    expect(dots).toHaveLength(3);
  });

  it('has accessible label', () => {
    const { container } = render(<TypingIndicator />);
    const wrapper = container.firstChild;
    expect(wrapper).toHaveAttribute('aria-label', 'AI is typing');
  });

  it('has aria-live attribute for screen readers', () => {
    const { container } = render(<TypingIndicator />);
    expect(container.firstChild).toHaveAttribute('aria-live', 'polite');
  });
});
