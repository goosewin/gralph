import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useLocalStorage } from './useLocalStorage';

describe('useLocalStorage', () => {
  const localStorageMock = (() => {
    let store: Record<string, string> = {};
    return {
      getItem: vi.fn((key: string) => store[key] ?? null),
      setItem: vi.fn((key: string, value: string) => {
        store[key] = value;
      }),
      removeItem: vi.fn((key: string) => {
        delete store[key];
      }),
      clear: vi.fn(() => {
        store = {};
      }),
    };
  })();

  beforeEach(() => {
    vi.stubGlobal('localStorage', localStorageMock);
    localStorageMock.clear();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns initial value when localStorage is empty', () => {
    const { result } = renderHook(() => useLocalStorage('test-key', 'initial'));

    expect(result.current[0]).toBe('initial');
  });

  it('returns stored value from localStorage', () => {
    localStorageMock.setItem('test-key', JSON.stringify('stored-value'));

    const { result } = renderHook(() => useLocalStorage('test-key', 'initial'));

    expect(result.current[0]).toBe('stored-value');
  });

  it('updates localStorage when setValue is called', () => {
    const { result } = renderHook(() => useLocalStorage('test-key', 'initial'));

    act(() => {
      result.current[1]('new-value');
    });

    expect(result.current[0]).toBe('new-value');
    expect(localStorageMock.setItem).toHaveBeenCalledWith(
      'test-key',
      JSON.stringify('new-value')
    );
  });

  it('supports functional updates', () => {
    const { result } = renderHook(() => useLocalStorage('counter', 0));

    act(() => {
      result.current[1]((prev) => prev + 1);
    });

    expect(result.current[0]).toBe(1);

    act(() => {
      result.current[1]((prev) => prev + 5);
    });

    expect(result.current[0]).toBe(6);
  });

  it('handles object values', () => {
    const initialValue = { name: 'test', count: 0 };
    const { result } = renderHook(() =>
      useLocalStorage('object-key', initialValue)
    );

    expect(result.current[0]).toEqual(initialValue);

    act(() => {
      result.current[1]({ name: 'updated', count: 5 });
    });

    expect(result.current[0]).toEqual({ name: 'updated', count: 5 });
  });

  it('handles array values', () => {
    const initialValue = ['a', 'b', 'c'];
    const { result } = renderHook(() =>
      useLocalStorage('array-key', initialValue)
    );

    expect(result.current[0]).toEqual(initialValue);

    act(() => {
      result.current[1](['x', 'y', 'z']);
    });

    expect(result.current[0]).toEqual(['x', 'y', 'z']);
  });

  it('returns initial value on JSON parse error', () => {
    localStorageMock.getItem.mockReturnValueOnce('invalid-json');

    const { result } = renderHook(() =>
      useLocalStorage('invalid-key', 'default')
    );

    expect(result.current[0]).toBe('default');
  });

  it('ignores localStorage errors on setItem', () => {
    localStorageMock.setItem.mockImplementationOnce(() => {
      throw new Error('QuotaExceededError');
    });

    const { result } = renderHook(() => useLocalStorage('test-key', 'initial'));

    // Should not throw
    act(() => {
      result.current[1]('new-value');
    });

    // Value should still update in memory
    expect(result.current[0]).toBe('new-value');
  });

  it('syncs with storage events from other tabs', () => {
    const { result } = renderHook(() => useLocalStorage('sync-key', 'initial'));

    expect(result.current[0]).toBe('initial');

    // Simulate storage event from another tab
    act(() => {
      const event = new StorageEvent('storage', {
        key: 'sync-key',
        newValue: JSON.stringify('synced-value'),
      });
      window.dispatchEvent(event);
    });

    expect(result.current[0]).toBe('synced-value');
  });

  it('ignores storage events for different keys', () => {
    const { result } = renderHook(() => useLocalStorage('my-key', 'initial'));

    act(() => {
      const event = new StorageEvent('storage', {
        key: 'other-key',
        newValue: JSON.stringify('other-value'),
      });
      window.dispatchEvent(event);
    });

    expect(result.current[0]).toBe('initial');
  });

  it('ignores storage events with null newValue', () => {
    const { result } = renderHook(() => useLocalStorage('test-key', 'initial'));

    act(() => {
      const event = new StorageEvent('storage', {
        key: 'test-key',
        newValue: null,
      });
      window.dispatchEvent(event);
    });

    expect(result.current[0]).toBe('initial');
  });

  it('ignores storage events with invalid JSON', () => {
    const { result } = renderHook(() => useLocalStorage('test-key', 'initial'));

    act(() => {
      const event = new StorageEvent('storage', {
        key: 'test-key',
        newValue: 'invalid-json{',
      });
      window.dispatchEvent(event);
    });

    expect(result.current[0]).toBe('initial');
  });

  it('cleans up storage event listener on unmount', () => {
    const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');

    const { unmount } = renderHook(() =>
      useLocalStorage('cleanup-key', 'initial')
    );

    unmount();

    expect(removeEventListenerSpy).toHaveBeenCalledWith(
      'storage',
      expect.any(Function)
    );

    removeEventListenerSpy.mockRestore();
  });

  it('maintains stable setValue reference', () => {
    const { result, rerender } = renderHook(() =>
      useLocalStorage('stable-key', 'initial')
    );

    const setValue1 = result.current[1];
    rerender();
    const setValue2 = result.current[1];

    expect(setValue1).toBe(setValue2);
  });

  it('handles boolean values', () => {
    const { result } = renderHook(() => useLocalStorage('bool-key', false));

    expect(result.current[0]).toBe(false);

    act(() => {
      result.current[1](true);
    });

    expect(result.current[0]).toBe(true);
  });

  it('handles null values', () => {
    const { result } = renderHook(() =>
      useLocalStorage<string | null>('null-key', null)
    );

    expect(result.current[0]).toBe(null);

    act(() => {
      result.current[1]('not-null');
    });

    expect(result.current[0]).toBe('not-null');

    act(() => {
      result.current[1](null);
    });

    expect(result.current[0]).toBe(null);
  });
});
