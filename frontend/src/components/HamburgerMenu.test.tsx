import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { HamburgerMenu } from './HamburgerMenu';

describe('HamburgerMenu', () => {
  it('renders a button with correct accessibility attributes when closed', () => {
    const onToggle = vi.fn();
    render(<HamburgerMenu isOpen={false} onToggle={onToggle} aria-controls="main-nav" />);

    const button = screen.getByRole('button');
    expect(button).toBeInTheDocument();
    expect(button).toHaveAttribute('aria-expanded', 'false');
    expect(button).toHaveAttribute('aria-controls', 'main-nav');
    expect(button).toHaveAttribute('aria-label', 'Open navigation menu');
  });

  it('renders with correct accessibility attributes when open', () => {
    const onToggle = vi.fn();
    render(<HamburgerMenu isOpen={true} onToggle={onToggle} />);

    const button = screen.getByRole('button');
    expect(button).toHaveAttribute('aria-expanded', 'true');
    expect(button).toHaveAttribute('aria-label', 'Close navigation menu');
  });

  it('calls onToggle when clicked', () => {
    const onToggle = vi.fn();
    render(<HamburgerMenu isOpen={false} onToggle={onToggle} />);

    const button = screen.getByRole('button');
    fireEvent.click(button);

    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it('renders three lines for hamburger icon', () => {
    const onToggle = vi.fn();
    const { container } = render(<HamburgerMenu isOpen={false} onToggle={onToggle} />);

    const lines = container.querySelectorAll('.hamburger-menu__line');
    expect(lines).toHaveLength(3);
  });

  it('applies open class when isOpen is true', () => {
    const onToggle = vi.fn();
    const { container } = render(<HamburgerMenu isOpen={true} onToggle={onToggle} />);

    const button = container.querySelector('.hamburger-menu');
    expect(button).toHaveClass('hamburger-menu--open');
  });

  it('does not apply open class when isOpen is false', () => {
    const onToggle = vi.fn();
    const { container } = render(<HamburgerMenu isOpen={false} onToggle={onToggle} />);

    const button = container.querySelector('.hamburger-menu');
    expect(button).not.toHaveClass('hamburger-menu--open');
  });

  it('has type button to prevent form submission', () => {
    const onToggle = vi.fn();
    render(<HamburgerMenu isOpen={false} onToggle={onToggle} />);

    const button = screen.getByRole('button');
    expect(button).toHaveAttribute('type', 'button');
  });

  it('lines have aria-hidden attribute', () => {
    const onToggle = vi.fn();
    const { container } = render(<HamburgerMenu isOpen={false} onToggle={onToggle} />);

    const lines = container.querySelectorAll('.hamburger-menu__line');
    lines.forEach((line) => {
      expect(line).toHaveAttribute('aria-hidden', 'true');
    });
  });
});
