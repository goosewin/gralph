import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useMediaQuery, useBreakpoints, BREAKPOINTS } from './useMediaQuery';

describe('useMediaQuery', () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    // Reset matchMedia mock
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
  });

  it('returns true when query matches', () => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: query === '(min-width: 768px)',
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    expect(result.current).toBe(true);
  });

  it('returns false when query does not match', () => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useMediaQuery('(min-width: 1024px)'));
    expect(result.current).toBe(false);
  });

  it('updates when media query changes', () => {
    let changeHandler: ((event: MediaQueryListEvent) => void) | null = null;

    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn((event: string, handler: (event: MediaQueryListEvent) => void) => {
        if (event === 'change') {
          changeHandler = handler;
        }
      }),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    expect(result.current).toBe(false);

    // Simulate media query change
    act(() => {
      if (changeHandler) {
        changeHandler({ matches: true } as MediaQueryListEvent);
      }
    });

    expect(result.current).toBe(true);
  });

  it('cleans up event listener on unmount', () => {
    const removeEventListener = vi.fn();

    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener,
      dispatchEvent: vi.fn(),
    }));

    const { unmount } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    unmount();

    expect(removeEventListener).toHaveBeenCalledWith('change', expect.any(Function));
  });

  it('handles legacy addListener API', () => {
    const addListener = vi.fn();
    const removeListener = vi.fn();

    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener,
      removeListener,
      // No addEventListener - simulating older browser
      dispatchEvent: vi.fn(),
    }));

    const { unmount } = renderHook(() => useMediaQuery('(min-width: 768px)'));

    expect(addListener).toHaveBeenCalled();

    unmount();
    expect(removeListener).toHaveBeenCalled();
  });
});

describe('useBreakpoints', () => {
  const originalMatchMedia = window.matchMedia;

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
  });

  it('returns correct breakpoint states for mobile view', () => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false, // All queries return false = mobile
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useBreakpoints());

    expect(result.current.isSm).toBe(false);
    expect(result.current.isMd).toBe(false);
    expect(result.current.isLg).toBe(false);
    expect(result.current.isMobile).toBe(true);
    expect(result.current.isTablet).toBe(false);
    expect(result.current.isDesktop).toBe(false);
  });

  it('returns correct breakpoint states for tablet view', () => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches:
        query === `(min-width: ${BREAKPOINTS.sm})` ||
        query === `(min-width: ${BREAKPOINTS.md})`,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useBreakpoints());

    expect(result.current.isSm).toBe(true);
    expect(result.current.isMd).toBe(true);
    expect(result.current.isLg).toBe(false);
    expect(result.current.isMobile).toBe(false);
    expect(result.current.isTablet).toBe(true);
    expect(result.current.isDesktop).toBe(false);
  });

  it('returns correct breakpoint states for desktop view', () => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches:
        query === `(min-width: ${BREAKPOINTS.sm})` ||
        query === `(min-width: ${BREAKPOINTS.md})` ||
        query === `(min-width: ${BREAKPOINTS.lg})`,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useBreakpoints());

    expect(result.current.isSm).toBe(true);
    expect(result.current.isMd).toBe(true);
    expect(result.current.isLg).toBe(true);
    expect(result.current.isMobile).toBe(false);
    expect(result.current.isTablet).toBe(false);
    expect(result.current.isDesktop).toBe(true);
  });

  it('detects touch device', () => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: query === '(hover: none) and (pointer: coarse)',
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useBreakpoints());

    expect(result.current.isTouchDevice).toBe(true);
  });

  it('correctly identifies non-touch device', () => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useBreakpoints());

    expect(result.current.isTouchDevice).toBe(false);
  });
});

describe('BREAKPOINTS', () => {
  it('exports correct breakpoint values', () => {
    expect(BREAKPOINTS.sm).toBe('640px');
    expect(BREAKPOINTS.md).toBe('768px');
    expect(BREAKPOINTS.lg).toBe('1024px');
  });
});
