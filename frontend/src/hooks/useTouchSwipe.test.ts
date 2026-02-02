import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useTouchSwipe } from './useTouchSwipe';

// Helper to create touch events
function createTouchEvent(_type: string, x: number, y: number) {
  return {
    touches: [{ clientX: x, clientY: y }],
    preventDefault: vi.fn(),
  } as unknown as React.TouchEvent;
}

describe('useTouchSwipe', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns handlers, swiping state, direction, and distance', () => {
    const { result } = renderHook(() => useTouchSwipe());

    expect(result.current.handlers).toBeDefined();
    expect(result.current.handlers.onTouchStart).toBeInstanceOf(Function);
    expect(result.current.handlers.onTouchMove).toBeInstanceOf(Function);
    expect(result.current.handlers.onTouchEnd).toBeInstanceOf(Function);
    expect(result.current.swiping).toBe(false);
    expect(result.current.direction).toBeNull();
    expect(result.current.distance).toBe(0);
  });

  it('sets swiping to true on touch start', () => {
    const { result } = renderHook(() => useTouchSwipe());

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    expect(result.current.swiping).toBe(true);
  });

  it('detects horizontal swipe direction on touch move', () => {
    const { result } = renderHook(() => useTouchSwipe());

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 150, 100));
    });

    expect(result.current.direction).toBe('right');
    expect(result.current.distance).toBe(50);
  });

  it('detects left swipe direction', () => {
    const { result } = renderHook(() => useTouchSwipe());

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 150, 100));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 100, 100));
    });

    expect(result.current.direction).toBe('left');
    expect(result.current.distance).toBe(50);
  });

  it('detects vertical swipe direction', () => {
    const { result } = renderHook(() => useTouchSwipe());

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 100, 150));
    });

    expect(result.current.direction).toBe('down');
    expect(result.current.distance).toBe(50);
  });

  it('calls onSwipe callback with direction on valid swipe', () => {
    const onSwipe = vi.fn();
    const { result } = renderHook(() =>
      useTouchSwipe({ onSwipe, minSwipeDistance: 50, maxSwipeTime: 300 })
    );

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 200, 100));
    });

    act(() => {
      result.current.handlers.onTouchEnd(createTouchEvent('touchend', 200, 100));
    });

    expect(onSwipe).toHaveBeenCalledWith('right');
  });

  it('calls onSwipeLeft callback on left swipe', () => {
    const onSwipeLeft = vi.fn();
    const { result } = renderHook(() =>
      useTouchSwipe({ onSwipeLeft, minSwipeDistance: 50, maxSwipeTime: 300 })
    );

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 200, 100));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 100, 100));
    });

    act(() => {
      result.current.handlers.onTouchEnd(createTouchEvent('touchend', 100, 100));
    });

    expect(onSwipeLeft).toHaveBeenCalledTimes(1);
  });

  it('calls onSwipeRight callback on right swipe', () => {
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() =>
      useTouchSwipe({ onSwipeRight, minSwipeDistance: 50, maxSwipeTime: 300 })
    );

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 200, 100));
    });

    act(() => {
      result.current.handlers.onTouchEnd(createTouchEvent('touchend', 200, 100));
    });

    expect(onSwipeRight).toHaveBeenCalledTimes(1);
  });

  it('does not trigger swipe if distance is too short', () => {
    const onSwipe = vi.fn();
    const { result } = renderHook(() =>
      useTouchSwipe({ onSwipe, minSwipeDistance: 100, maxSwipeTime: 300 })
    );

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 140, 100));
    });

    act(() => {
      result.current.handlers.onTouchEnd(createTouchEvent('touchend', 140, 100));
    });

    expect(onSwipe).not.toHaveBeenCalled();
  });

  it('does not trigger swipe if time exceeds maxSwipeTime', () => {
    const onSwipe = vi.fn();
    const { result } = renderHook(() =>
      useTouchSwipe({ onSwipe, minSwipeDistance: 50, maxSwipeTime: 300 })
    );

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    // Advance time past maxSwipeTime
    act(() => {
      vi.advanceTimersByTime(400);
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 200, 100));
    });

    act(() => {
      result.current.handlers.onTouchEnd(createTouchEvent('touchend', 200, 100));
    });

    expect(onSwipe).not.toHaveBeenCalled();
  });

  it('does not trigger when disabled', () => {
    const onSwipe = vi.fn();
    const { result } = renderHook(() =>
      useTouchSwipe({ onSwipe, enabled: false, minSwipeDistance: 50 })
    );

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    expect(result.current.swiping).toBe(false);

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 200, 100));
    });

    act(() => {
      result.current.handlers.onTouchEnd(createTouchEvent('touchend', 200, 100));
    });

    expect(onSwipe).not.toHaveBeenCalled();
  });

  it('resets state on touch end', () => {
    const { result } = renderHook(() => useTouchSwipe());

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 200, 100));
    });

    expect(result.current.swiping).toBe(true);
    expect(result.current.direction).toBe('right');

    act(() => {
      result.current.handlers.onTouchEnd(createTouchEvent('touchend', 200, 100));
    });

    expect(result.current.swiping).toBe(false);
    expect(result.current.direction).toBeNull();
    expect(result.current.distance).toBe(0);
  });

  it('uses default config values', () => {
    const onSwipe = vi.fn();
    const { result } = renderHook(() => useTouchSwipe({ onSwipe }));

    // Default minSwipeDistance is 50
    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 160, 100));
    });

    act(() => {
      result.current.handlers.onTouchEnd(createTouchEvent('touchend', 160, 100));
    });

    expect(onSwipe).toHaveBeenCalledWith('right');
  });

  it('detects up swipe', () => {
    const onSwipe = vi.fn();
    const { result } = renderHook(() =>
      useTouchSwipe({ onSwipe, minSwipeDistance: 50 })
    );

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 150));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 100, 50));
    });

    act(() => {
      result.current.handlers.onTouchEnd(createTouchEvent('touchend', 100, 50));
    });

    expect(onSwipe).toHaveBeenCalledWith('up');
  });

  it('handles touch end without prior touch start gracefully', () => {
    const { result } = renderHook(() => useTouchSwipe());

    // Should not throw
    expect(() => {
      act(() => {
        result.current.handlers.onTouchEnd(createTouchEvent('touchend', 100, 100));
      });
    }).not.toThrow();

    expect(result.current.swiping).toBe(false);
  });

  it('ignores small movements that do not meet threshold', () => {
    const { result } = renderHook(() => useTouchSwipe());

    act(() => {
      result.current.handlers.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    act(() => {
      result.current.handlers.onTouchMove(createTouchEvent('touchmove', 105, 103));
    });

    // Direction should still be null for tiny movements
    expect(result.current.direction).toBeNull();
  });
});
