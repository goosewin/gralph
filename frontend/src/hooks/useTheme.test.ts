import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useTheme } from './useTheme';

describe('useTheme', () => {
  const originalMatchMedia = window.matchMedia;
  const originalLocalStorage = window.localStorage;

  beforeEach(() => {
    // Reset localStorage mock
    const store: Record<string, string> = {};
    const mockLocalStorage = {
      getItem: vi.fn((key: string) => store[key] ?? null),
      setItem: vi.fn((key: string, value: string) => {
        store[key] = value;
      }),
      removeItem: vi.fn((key: string) => {
        delete store[key];
      }),
      clear: vi.fn(() => {
        Object.keys(store).forEach((key) => delete store[key]);
      }),
      length: 0,
      key: vi.fn(),
    };
    Object.defineProperty(window, 'localStorage', { value: mockLocalStorage, writable: true });

    // Reset matchMedia mock to default dark preference
    Object.defineProperty(window, 'matchMedia', {
      value: vi.fn((query: string) => ({
        matches: query === '(prefers-color-scheme: dark)',
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
      })),
      writable: true,
    });

    // Clean up document attribute
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.style.colorScheme = '';
  });

  afterEach(() => {
    Object.defineProperty(window, 'matchMedia', { value: originalMatchMedia, writable: true });
    Object.defineProperty(window, 'localStorage', { value: originalLocalStorage, writable: true });
  });

  it('defaults to system theme when no stored preference', () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe('system');
  });

  it('resolves system theme to dark when system prefers dark', () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe('system');
    expect(result.current.resolvedTheme).toBe('dark');
  });

  it('resolves system theme to light when system prefers light', () => {
    Object.defineProperty(window, 'matchMedia', {
      value: vi.fn((query: string) => ({
        matches: query !== '(prefers-color-scheme: dark)',
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      })),
      writable: true,
    });

    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe('system');
    expect(result.current.resolvedTheme).toBe('light');
  });

  it('loads theme from localStorage', () => {
    localStorage.setItem('gralph-theme', 'light');
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe('light');
    expect(result.current.resolvedTheme).toBe('light');
  });

  it('loads dark theme from localStorage', () => {
    localStorage.setItem('gralph-theme', 'dark');
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe('dark');
    expect(result.current.resolvedTheme).toBe('dark');
  });

  it('ignores invalid stored theme values', () => {
    localStorage.setItem('gralph-theme', 'invalid');
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe('system');
  });

  it('saves theme to localStorage when changed', () => {
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme('dark');
    });

    expect(localStorage.setItem).toHaveBeenCalledWith('gralph-theme', 'dark');
    expect(result.current.theme).toBe('dark');
  });

  it('updates document data-theme attribute', () => {
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme('light');
    });

    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });

  it('updates document color-scheme style', () => {
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme('dark');
    });

    expect(document.documentElement.style.colorScheme).toBe('dark');
  });

  it('resolvedTheme equals explicit theme when not system', () => {
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme('light');
    });

    expect(result.current.resolvedTheme).toBe('light');

    act(() => {
      result.current.setTheme('dark');
    });

    expect(result.current.resolvedTheme).toBe('dark');
  });

  it('setTheme can switch between all themes', () => {
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme('light');
    });
    expect(result.current.theme).toBe('light');

    act(() => {
      result.current.setTheme('dark');
    });
    expect(result.current.theme).toBe('dark');

    act(() => {
      result.current.setTheme('system');
    });
    expect(result.current.theme).toBe('system');
  });

  it('applies theme to document on mount', () => {
    localStorage.setItem('gralph-theme', 'dark');
    renderHook(() => useTheme());

    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    expect(document.documentElement.style.colorScheme).toBe('dark');
  });

  it('listens for system theme changes', () => {
    let eventCallback: ((e: MediaQueryListEvent) => void) | null = null;

    Object.defineProperty(window, 'matchMedia', {
      value: vi.fn((query: string) => ({
        matches: query === '(prefers-color-scheme: dark)',
        media: query,
        addEventListener: vi.fn((event: string, cb: (e: MediaQueryListEvent) => void) => {
          if (event === 'change') {
            eventCallback = cb;
          }
        }),
        removeEventListener: vi.fn(),
      })),
      writable: true,
    });

    const { result } = renderHook(() => useTheme());

    expect(result.current.resolvedTheme).toBe('dark');

    // Simulate system preference change
    if (eventCallback) {
      act(() => {
        eventCallback!({ matches: false } as MediaQueryListEvent);
      });
    }

    expect(result.current.resolvedTheme).toBe('light');
  });
});
