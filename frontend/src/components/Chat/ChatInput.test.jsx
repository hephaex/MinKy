import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ChatInput from './ChatInput';

describe('ChatInput', () => {
  it('renders textarea and send button', () => {
    render(<ChatInput onSend={jest.fn()} />);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /send message/i })).toBeInTheDocument();
  });

  it('send button is disabled when input is empty', () => {
    render(<ChatInput onSend={jest.fn()} />);
    expect(screen.getByRole('button', { name: /send message/i })).toBeDisabled();
  });

  it('send button is enabled when input has text', async () => {
    render(<ChatInput onSend={jest.fn()} />);
    await userEvent.type(screen.getByRole('textbox'), 'Hello');
    expect(screen.getByRole('button', { name: /send message/i })).not.toBeDisabled();
  });

  it('calls onSend with trimmed value on button click', async () => {
    const onSend = jest.fn();
    render(<ChatInput onSend={onSend} />);
    await userEvent.type(screen.getByRole('textbox'), '  hello  ');
    fireEvent.click(screen.getByRole('button', { name: /send message/i }));
    expect(onSend).toHaveBeenCalledWith('hello');
  });

  it('calls onSend and clears input on Enter key', async () => {
    const onSend = jest.fn();
    render(<ChatInput onSend={onSend} />);
    const textarea = screen.getByRole('textbox');
    await userEvent.type(textarea, 'test message');
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });
    expect(onSend).toHaveBeenCalledWith('test message');
  });

  it('does not submit on Shift+Enter', async () => {
    const onSend = jest.fn();
    render(<ChatInput onSend={onSend} />);
    const textarea = screen.getByRole('textbox');
    await userEvent.type(textarea, 'line one');
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: true });
    expect(onSend).not.toHaveBeenCalled();
  });

  it('disables textarea and button when disabled prop is true', () => {
    render(<ChatInput onSend={jest.fn()} disabled />);
    expect(screen.getByRole('textbox')).toBeDisabled();
    expect(screen.getByRole('button', { name: /send message/i })).toBeDisabled();
  });

  it('does not call onSend when disabled', async () => {
    const onSend = jest.fn();
    render(<ChatInput onSend={onSend} disabled />);
    const textarea = screen.getByRole('textbox');
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });
    expect(onSend).not.toHaveBeenCalled();
  });
});
