import { useCallback, useEffect, useState } from 'react';

export type ServiceWorkerStatus = 'idle' | 'registering' | 'ready' | 'error';

export interface UseServiceWorkerResult {
  /** Current status of the service worker */
  status: ServiceWorkerStatus;
  /** Whether the service worker is registered and ready */
  isReady: boolean;
  /** Whether an update is available */
  updateAvailable: boolean;
  /** Error message if registration failed */
  error: string | null;
  /** Apply available update by reloading */
  applyUpdate: () => void;
}

/**
 * Hook to manage service worker registration and updates.
 * Registers the service worker on mount and listens for updates.
 */
export function useServiceWorker(): UseServiceWorkerResult {
  const [status, setStatus] = useState<ServiceWorkerStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [waitingWorker, setWaitingWorker] = useState<ServiceWorker | null>(null);

  useEffect(() => {
    // Skip if service workers aren't supported
    if (!('serviceWorker' in navigator) || !navigator.serviceWorker) {
      return;
    }

    const sw = navigator.serviceWorker;

    const registerServiceWorker = async () => {
      setStatus('registering');

      try {
        const registration = await sw.register('/sw.js', {
          scope: '/',
        });

        // Handle updates
        registration.addEventListener('updatefound', () => {
          const newWorker = registration.installing;
          if (newWorker) {
            newWorker.addEventListener('statechange', () => {
              if (newWorker.state === 'installed' && sw.controller) {
                // New service worker is ready, prompt for update
                setUpdateAvailable(true);
                setWaitingWorker(newWorker);
              }
            });
          }
        });

        // Check if there's already a waiting worker
        if (registration.waiting) {
          setUpdateAvailable(true);
          setWaitingWorker(registration.waiting);
        }

        setStatus('ready');
      } catch (err) {
        setStatus('error');
        setError(err instanceof Error ? err.message : 'Failed to register service worker');
      }
    };

    registerServiceWorker();

    // Listen for controller change (after skip waiting)
    const handleControllerChange = () => {
      window.location.reload();
    };

    sw.addEventListener('controllerchange', handleControllerChange);

    return () => {
      sw.removeEventListener('controllerchange', handleControllerChange);
    };
  }, []);

  const applyUpdate = useCallback(() => {
    if (waitingWorker) {
      // Tell the waiting worker to skip waiting
      waitingWorker.postMessage({ type: 'SKIP_WAITING' });
    }
  }, [waitingWorker]);

  return {
    status,
    isReady: status === 'ready',
    updateAvailable,
    error,
    applyUpdate,
  };
}
