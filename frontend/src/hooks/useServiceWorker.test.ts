import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useServiceWorker } from './useServiceWorker';

describe('useServiceWorker', () => {
  let originalServiceWorker: ServiceWorkerContainer | undefined;
  let mockRegistration: ServiceWorkerRegistration;
  let mockServiceWorker: ServiceWorker;

  beforeEach(() => {
    // Store original
    originalServiceWorker = navigator.serviceWorker;

    // Create mock service worker
    mockServiceWorker = {
      addEventListener: vi.fn(),
      postMessage: vi.fn(),
      state: 'activated',
    } as unknown as ServiceWorker;

    // Create mock registration
    mockRegistration = {
      addEventListener: vi.fn(),
      installing: null,
      waiting: null,
      active: mockServiceWorker,
      scope: '/',
      updateViaCache: 'none',
      update: vi.fn(),
      unregister: vi.fn(),
    } as unknown as ServiceWorkerRegistration;
  });

  afterEach(() => {
    // Restore original
    if (originalServiceWorker) {
      Object.defineProperty(navigator, 'serviceWorker', {
        value: originalServiceWorker,
        configurable: true,
      });
    }
    vi.restoreAllMocks();
  });

  it('returns idle status when service workers are not supported', () => {
    // Remove serviceWorker from navigator
    Object.defineProperty(navigator, 'serviceWorker', {
      value: undefined,
      configurable: true,
    });

    const { result } = renderHook(() => useServiceWorker());

    expect(result.current.status).toBe('idle');
    expect(result.current.isReady).toBe(false);
    expect(result.current.updateAvailable).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('registers service worker on mount', async () => {
    const registerMock = vi.fn().mockResolvedValue(mockRegistration);

    Object.defineProperty(navigator, 'serviceWorker', {
      value: {
        register: registerMock,
        controller: mockServiceWorker,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      },
      configurable: true,
    });

    const { result } = renderHook(() => useServiceWorker());

    await waitFor(() => {
      expect(result.current.status).toBe('ready');
    });

    expect(registerMock).toHaveBeenCalledWith('/sw.js', { scope: '/' });
    expect(result.current.isReady).toBe(true);
  });

  it('handles registration error', async () => {
    const error = new Error('Registration failed');
    const registerMock = vi.fn().mockRejectedValue(error);

    Object.defineProperty(navigator, 'serviceWorker', {
      value: {
        register: registerMock,
        controller: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      },
      configurable: true,
    });

    const { result } = renderHook(() => useServiceWorker());

    await waitFor(() => {
      expect(result.current.status).toBe('error');
    });

    expect(result.current.error).toBe('Registration failed');
    expect(result.current.isReady).toBe(false);
  });

  it('detects waiting worker and sets updateAvailable', async () => {
    const waitingWorker = {
      addEventListener: vi.fn(),
      postMessage: vi.fn(),
      state: 'installed',
    } as unknown as ServiceWorker;

    mockRegistration.waiting = waitingWorker;
    const registerMock = vi.fn().mockResolvedValue(mockRegistration);

    Object.defineProperty(navigator, 'serviceWorker', {
      value: {
        register: registerMock,
        controller: mockServiceWorker,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      },
      configurable: true,
    });

    const { result } = renderHook(() => useServiceWorker());

    await waitFor(() => {
      expect(result.current.updateAvailable).toBe(true);
    });
  });

  it('applyUpdate sends SKIP_WAITING message to waiting worker', async () => {
    const waitingWorker = {
      addEventListener: vi.fn(),
      postMessage: vi.fn(),
      state: 'installed',
    } as unknown as ServiceWorker;

    mockRegistration.waiting = waitingWorker;
    const registerMock = vi.fn().mockResolvedValue(mockRegistration);

    Object.defineProperty(navigator, 'serviceWorker', {
      value: {
        register: registerMock,
        controller: mockServiceWorker,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      },
      configurable: true,
    });

    const { result } = renderHook(() => useServiceWorker());

    await waitFor(() => {
      expect(result.current.updateAvailable).toBe(true);
    });

    act(() => {
      result.current.applyUpdate();
    });

    expect(waitingWorker.postMessage).toHaveBeenCalledWith({ type: 'SKIP_WAITING' });
  });

  it('handles updatefound event and tracks installing worker', async () => {
    let updateFoundCallback: (() => void) | null = null;
    let stateChangeCallback: (() => void) | null = null;

    const installingWorker = {
      addEventListener: vi.fn((event, callback) => {
        if (event === 'statechange') {
          stateChangeCallback = callback;
        }
      }),
      postMessage: vi.fn(),
      state: 'installed',
    } as unknown as ServiceWorker;

    mockRegistration.addEventListener = vi.fn((event, callback) => {
      if (event === 'updatefound') {
        updateFoundCallback = callback;
      }
    });

    const registerMock = vi.fn().mockResolvedValue(mockRegistration);

    Object.defineProperty(navigator, 'serviceWorker', {
      value: {
        register: registerMock,
        controller: mockServiceWorker,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      },
      configurable: true,
    });

    const { result } = renderHook(() => useServiceWorker());

    await waitFor(() => {
      expect(result.current.status).toBe('ready');
    });

    // Simulate updatefound event
    mockRegistration.installing = installingWorker;
    act(() => {
      updateFoundCallback?.();
    });

    // Simulate worker installed state change
    act(() => {
      stateChangeCallback?.();
    });

    await waitFor(() => {
      expect(result.current.updateAvailable).toBe(true);
    });
  });

  it('sets up controllerchange listener', async () => {
    let controllerChangeCallback: (() => void) | null = null;

    const registerMock = vi.fn().mockResolvedValue(mockRegistration);
    const addEventListenerMock = vi.fn((event, callback) => {
      if (event === 'controllerchange') {
        controllerChangeCallback = callback;
      }
    });

    Object.defineProperty(navigator, 'serviceWorker', {
      value: {
        register: registerMock,
        controller: mockServiceWorker,
        addEventListener: addEventListenerMock,
        removeEventListener: vi.fn(),
      },
      configurable: true,
    });

    renderHook(() => useServiceWorker());

    await waitFor(() => {
      expect(addEventListenerMock).toHaveBeenCalledWith('controllerchange', expect.any(Function));
    });

    expect(controllerChangeCallback).not.toBeNull();
  });

  it('cleans up controllerchange listener on unmount', async () => {
    const registerMock = vi.fn().mockResolvedValue(mockRegistration);
    const removeEventListenerMock = vi.fn();

    Object.defineProperty(navigator, 'serviceWorker', {
      value: {
        register: registerMock,
        controller: mockServiceWorker,
        addEventListener: vi.fn(),
        removeEventListener: removeEventListenerMock,
      },
      configurable: true,
    });

    const { unmount } = renderHook(() => useServiceWorker());

    await waitFor(() => {
      // Wait for registration to complete
    });

    unmount();

    expect(removeEventListenerMock).toHaveBeenCalledWith('controllerchange', expect.any(Function));
  });

  it('handles non-Error objects in catch block', async () => {
    const registerMock = vi.fn().mockRejectedValue('String error');

    Object.defineProperty(navigator, 'serviceWorker', {
      value: {
        register: registerMock,
        controller: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      },
      configurable: true,
    });

    const { result } = renderHook(() => useServiceWorker());

    await waitFor(() => {
      expect(result.current.status).toBe('error');
    });

    expect(result.current.error).toBe('Failed to register service worker');
  });
});
