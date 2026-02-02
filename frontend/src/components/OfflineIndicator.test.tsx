import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { OfflineIndicator } from './OfflineIndicator';

describe('OfflineIndicator', () => {
  let originalOnLine: boolean;

  beforeEach(() => {
    // Store original value
    originalOnLine = navigator.onLine;
  });

  afterEach(() => {
    // Restore original value
    Object.defineProperty(navigator, 'onLine', {
      value: originalOnLine,
      configurable: true,
      writable: true,
    });
    vi.restoreAllMocks();
  });

  it('renders nothing when online', () => {
    Object.defineProperty(navigator, 'onLine', {
      value: true,
      configurable: true,
    });

    const { container } = render(<OfflineIndicator />);
    expect(container.firstChild).toBeNull();
  });

  it('renders offline indicator when offline', () => {
    Object.defineProperty(navigator, 'onLine', {
      value: false,
      configurable: true,
    });

    render(<OfflineIndicator />);
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(/you are offline/i)).toBeInTheDocument();
  });

  it('has correct ARIA attributes', () => {
    Object.defineProperty(navigator, 'onLine', {
      value: false,
      configurable: true,
    });

    render(<OfflineIndicator />);
    const alert = screen.getByRole('alert');
    expect(alert).toHaveAttribute('aria-live', 'polite');
  });

  it('displays warning icon', () => {
    Object.defineProperty(navigator, 'onLine', {
      value: false,
      configurable: true,
    });

    render(<OfflineIndicator />);
    const icon = document.querySelector('.offline-indicator__icon');
    expect(icon).toBeInTheDocument();
    expect(icon).toHaveAttribute('aria-hidden', 'true');
  });

  it('applies custom className', () => {
    Object.defineProperty(navigator, 'onLine', {
      value: false,
      configurable: true,
    });

    render(<OfflineIndicator className="custom-class" />);
    const alert = screen.getByRole('alert');
    expect(alert).toHaveClass('offline-indicator', 'custom-class');
  });

  it('responds to online event', () => {
    Object.defineProperty(navigator, 'onLine', {
      value: false,
      configurable: true,
    });

    const { container } = render(<OfflineIndicator />);
    expect(screen.getByRole('alert')).toBeInTheDocument();

    // Simulate going online
    act(() => {
      window.dispatchEvent(new Event('online'));
    });

    expect(container.firstChild).toBeNull();
  });

  it('responds to offline event', () => {
    Object.defineProperty(navigator, 'onLine', {
      value: true,
      configurable: true,
    });

    const { container } = render(<OfflineIndicator />);
    expect(container.firstChild).toBeNull();

    // Simulate going offline
    act(() => {
      window.dispatchEvent(new Event('offline'));
    });

    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('cleans up event listeners on unmount', () => {
    const addEventListenerSpy = vi.spyOn(window, 'addEventListener');
    const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');

    Object.defineProperty(navigator, 'onLine', {
      value: false,
      configurable: true,
    });

    const { unmount } = render(<OfflineIndicator />);

    expect(addEventListenerSpy).toHaveBeenCalledWith('online', expect.any(Function));
    expect(addEventListenerSpy).toHaveBeenCalledWith('offline', expect.any(Function));

    unmount();

    expect(removeEventListenerSpy).toHaveBeenCalledWith('online', expect.any(Function));
    expect(removeEventListenerSpy).toHaveBeenCalledWith('offline', expect.any(Function));
  });

  it('has proper CSS class structure', () => {
    Object.defineProperty(navigator, 'onLine', {
      value: false,
      configurable: true,
    });

    render(<OfflineIndicator />);
    expect(document.querySelector('.offline-indicator')).toBeInTheDocument();
    expect(document.querySelector('.offline-indicator__icon')).toBeInTheDocument();
    expect(document.querySelector('.offline-indicator__text')).toBeInTheDocument();
  });
});
