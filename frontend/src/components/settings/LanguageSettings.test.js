import React from 'react';
import { render, screen } from '@testing-library/react';
import LanguageSettings from './LanguageSettings';
import * as i18n from '../../i18n/i18n';

jest.mock('../../i18n/i18n', () => ({
  LanguageSelector: () => <div data-testid="language-selector">Language Selector</div>
}));

describe('LanguageSettings', () => {
  it('renders language settings section', () => {
    render(<LanguageSettings />);
    expect(screen.getByText('Language Settings')).toBeInTheDocument();
  });

  it('displays description text', () => {
    render(<LanguageSettings />);
    expect(screen.getByText(/Select your preferred language/)).toBeInTheDocument();
  });

  it('renders language selector component', () => {
    render(<LanguageSettings />);
    expect(screen.getByTestId('language-selector')).toBeInTheDocument();
  });

  it('has label for language setting', () => {
    render(<LanguageSettings />);
    expect(screen.getByText('Interface Language:')).toBeInTheDocument();
  });

  it('renders in a settings section container', () => {
    const { container } = render(<LanguageSettings />);
    expect(container.querySelector('.settings-section')).toBeInTheDocument();
  });

  it('renders language setting div', () => {
    const { container } = render(<LanguageSettings />);
    expect(container.querySelector('.language-setting')).toBeInTheDocument();
  });
});
