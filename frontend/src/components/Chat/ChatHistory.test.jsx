import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import ChatHistory from './ChatHistory';

const sessions = [
  { id: 's1', title: 'Session One', updatedAt: '2026-02-19T09:00:00Z' },
  { id: 's2', title: 'Session Two', updatedAt: '2026-02-19T10:00:00Z' },
];

describe('ChatHistory', () => {
  it('renders empty state with New Chat button', () => {
    render(
      <ChatHistory sessions={[]} onSelect={jest.fn()} onNew={jest.fn()} />
    );
    expect(screen.getByText('No conversations yet')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /new chat/i })).toBeInTheDocument();
  });

  it('renders session list items', () => {
    render(
      <ChatHistory sessions={sessions} onSelect={jest.fn()} onNew={jest.fn()} />
    );
    expect(screen.getByText('Session One')).toBeInTheDocument();
    expect(screen.getByText('Session Two')).toBeInTheDocument();
  });

  it('calls onSelect when a session is clicked', () => {
    const onSelect = jest.fn();
    render(
      <ChatHistory sessions={sessions} onSelect={onSelect} onNew={jest.fn()} />
    );
    fireEvent.click(screen.getByText('Session One'));
    expect(onSelect).toHaveBeenCalledWith('s1');
  });

  it('calls onNew when New button is clicked', () => {
    const onNew = jest.fn();
    render(
      <ChatHistory sessions={sessions} onSelect={jest.fn()} onNew={onNew} />
    );
    fireEvent.click(screen.getByText('+ New'));
    expect(onNew).toHaveBeenCalled();
  });

  it('marks active session', () => {
    render(
      <ChatHistory
        sessions={sessions}
        activeId="s1"
        onSelect={jest.fn()}
        onNew={jest.fn()}
      />
    );
    const active = screen.getByText('Session One').closest('li');
    expect(active).toHaveClass('chat-history__item--active');
  });

  it('calls onDelete when delete button is clicked', () => {
    const onDelete = jest.fn();
    render(
      <ChatHistory
        sessions={sessions}
        onSelect={jest.fn()}
        onNew={jest.fn()}
        onDelete={onDelete}
      />
    );
    const deleteButtons = screen.getAllByRole('button', { name: /delete/i });
    fireEvent.click(deleteButtons[0]);
    expect(onDelete).toHaveBeenCalledWith('s1');
  });
});
