import { useCallback, useEffect, useState } from 'react';

// Type for the beforeinstallprompt event
interface BeforeInstallPromptEvent extends Event {
  readonly platforms: string[];
  readonly userChoice: Promise<{
    outcome: 'accepted' | 'dismissed';
    platform: string;
  }>;
  prompt(): Promise<void>;
}

// Extend WindowEventMap for TypeScript
declare global {
  interface WindowEventMap {
    beforeinstallprompt: BeforeInstallPromptEvent;
  }
}

export interface InstallPromptProps {
  /** Custom class name */
  className?: string;
}

/**
 * InstallPrompt shows a prompt to install the PWA on mobile devices.
 * Only displays when the beforeinstallprompt event fires (browser determines eligibility).
 */
export function InstallPrompt({ className = '' }: InstallPromptProps) {
  const [deferredPrompt, setDeferredPrompt] = useState<BeforeInstallPromptEvent | null>(null);
  const [isVisible, setIsVisible] = useState(false);
  const [isInstalled, setIsInstalled] = useState(false);

  // Track whether install was dismissed this session
  const [isDismissedSession] = useState(() =>
    sessionStorage.getItem('pwa-install-dismissed') === 'true'
  );

  useEffect(() => {
    // Check if already installed (standalone mode)
    if (window.matchMedia('(display-mode: standalone)').matches) {
      setIsInstalled(true);
      return;
    }

    // Don't set up listeners if previously dismissed
    if (isDismissedSession) {
      return;
    }

    const handleBeforeInstallPrompt = (e: BeforeInstallPromptEvent) => {
      // Prevent default browser prompt
      e.preventDefault();
      // Store the event for later use
      setDeferredPrompt(e);
      setIsVisible(true);
    };

    const handleAppInstalled = () => {
      setIsInstalled(true);
      setIsVisible(false);
      setDeferredPrompt(null);
    };

    window.addEventListener('beforeinstallprompt', handleBeforeInstallPrompt);
    window.addEventListener('appinstalled', handleAppInstalled);

    return () => {
      window.removeEventListener('beforeinstallprompt', handleBeforeInstallPrompt);
      window.removeEventListener('appinstalled', handleAppInstalled);
    };
  }, [isDismissedSession]);

  const handleInstall = useCallback(async () => {
    if (!deferredPrompt) return;

    // Show the install prompt
    await deferredPrompt.prompt();

    // Wait for the user's choice
    const { outcome } = await deferredPrompt.userChoice;

    if (outcome === 'accepted') {
      setIsVisible(false);
    }

    // Clear the deferred prompt
    setDeferredPrompt(null);
  }, [deferredPrompt]);

  const handleDismiss = useCallback(() => {
    setIsVisible(false);
    // Store dismissal in sessionStorage so we don't show again this session
    sessionStorage.setItem('pwa-install-dismissed', 'true');
  }, []);

  if (!isVisible || isInstalled || isDismissedSession) {
    return null;
  }

  return (
    <div
      className={`install-prompt ${className}`.trim()}
      role="complementary"
      aria-label="App installation prompt"
    >
      <div className="install-prompt__content">
        <span className="install-prompt__icon" aria-hidden="true">
          &#x1F4F1;
        </span>
        <div className="install-prompt__text">
          <strong>Install Gralph Mission Control</strong>
          <span>Add to your home screen for quick access</span>
        </div>
      </div>
      <div className="install-prompt__actions">
        <button
          className="install-prompt__button install-prompt__button--dismiss"
          onClick={handleDismiss}
          type="button"
        >
          Not now
        </button>
        <button
          className="install-prompt__button install-prompt__button--install"
          onClick={handleInstall}
          type="button"
        >
          Install
        </button>
      </div>
    </div>
  );
}
