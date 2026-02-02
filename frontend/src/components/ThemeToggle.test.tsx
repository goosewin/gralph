import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ThemeToggle } from './ThemeToggle';

describe('ThemeToggle', () => {
  it('renders all three theme options', () => {
    const onThemeChange = vi.fn();
    render(<ThemeToggle theme="system" onThemeChange={onThemeChange} />);

    expect(screen.getByTitle('Light')).toBeInTheDocument();
    expect(screen.getByTitle('Dark')).toBeInTheDocument();
    expect(screen.getByTitle('System')).toBeInTheDocument();
  });

  it('marks the current theme as active', () => {
    const onThemeChange = vi.fn();
    render(<ThemeToggle theme="dark" onThemeChange={onThemeChange} />);

    const darkButton = screen.getByTitle('Dark');
    expect(darkButton).toHaveClass('theme-toggle__button--active');
    expect(darkButton).toHaveAttribute('aria-pressed', 'true');
  });

  it('marks light theme as active when selected', () => {
    const onThemeChange = vi.fn();
    render(<ThemeToggle theme="light" onThemeChange={onThemeChange} />);

    const lightButton = screen.getByTitle('Light');
    expect(lightButton).toHaveClass('theme-toggle__button--active');
    expect(lightButton).toHaveAttribute('aria-pressed', 'true');
  });

  it('marks system theme as active when selected', () => {
    const onThemeChange = vi.fn();
    render(<ThemeToggle theme="system" onThemeChange={onThemeChange} />);

    const systemButton = screen.getByTitle('System');
    expect(systemButton).toHaveClass('theme-toggle__button--active');
    expect(systemButton).toHaveAttribute('aria-pressed', 'true');
  });

  it('calls onThemeChange when light button is clicked', () => {
    const onThemeChange = vi.fn();
    render(<ThemeToggle theme="dark" onThemeChange={onThemeChange} />);

    fireEvent.click(screen.getByTitle('Light'));
    expect(onThemeChange).toHaveBeenCalledWith('light');
  });

  it('calls onThemeChange when dark button is clicked', () => {
    const onThemeChange = vi.fn();
    render(<ThemeToggle theme="light" onThemeChange={onThemeChange} />);

    fireEvent.click(screen.getByTitle('Dark'));
    expect(onThemeChange).toHaveBeenCalledWith('dark');
  });

  it('calls onThemeChange when system button is clicked', () => {
    const onThemeChange = vi.fn();
    render(<ThemeToggle theme="dark" onThemeChange={onThemeChange} />);

    fireEvent.click(screen.getByTitle('System'));
    expect(onThemeChange).toHaveBeenCalledWith('system');
  });

  it('has accessible group role', () => {
    const onThemeChange = vi.fn();
    render(<ThemeToggle theme="system" onThemeChange={onThemeChange} />);

    expect(screen.getByRole('group')).toHaveAttribute('aria-label', 'Theme selection');
  });

  it('has screen reader text for each button', () => {
    const onThemeChange = vi.fn();
    render(<ThemeToggle theme="system" onThemeChange={onThemeChange} />);

    expect(screen.getByText('Light theme')).toHaveClass('sr-only');
    expect(screen.getByText('Dark theme')).toHaveClass('sr-only');
    expect(screen.getByText('System theme')).toHaveClass('sr-only');
  });

  it('non-active buttons are not pressed', () => {
    const onThemeChange = vi.fn();
    render(<ThemeToggle theme="dark" onThemeChange={onThemeChange} />);

    const lightButton = screen.getByTitle('Light');
    const systemButton = screen.getByTitle('System');

    expect(lightButton).toHaveAttribute('aria-pressed', 'false');
    expect(systemButton).toHaveAttribute('aria-pressed', 'false');
    expect(lightButton).not.toHaveClass('theme-toggle__button--active');
    expect(systemButton).not.toHaveClass('theme-toggle__button--active');
  });
});
