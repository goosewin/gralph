import { useEffect, useState } from 'react';

export interface OfflineIndicatorProps {
  /** Custom class name */
  className?: string;
}

/**
 * OfflineIndicator displays a banner when the user is offline.
 * Uses the Navigator.onLine API with online/offline events for real-time updates.
 */
export function OfflineIndicator({ className = '' }: OfflineIndicatorProps) {
  const [isOffline, setIsOffline] = useState(!navigator.onLine);

  useEffect(() => {
    const handleOnline = () => setIsOffline(false);
    const handleOffline = () => setIsOffline(true);

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  if (!isOffline) {
    return null;
  }

  return (
    <div
      className={`offline-indicator ${className}`.trim()}
      role="alert"
      aria-live="polite"
    >
      <span className="offline-indicator__icon" aria-hidden="true">
        &#x26A0;
      </span>
      <span className="offline-indicator__text">
        You are offline. Some features may be unavailable.
      </span>
    </div>
  );
}
