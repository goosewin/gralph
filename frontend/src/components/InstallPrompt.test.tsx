import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { InstallPrompt } from './InstallPrompt';

// Mock BeforeInstallPromptEvent
interface MockBeforeInstallPromptEvent extends Event {
  platforms: string[];
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed'; platform: string }>;
  prompt: () => Promise<void>;
}

function createMockInstallEvent(outcome: 'accepted' | 'dismissed' = 'accepted'): MockBeforeInstallPromptEvent {
  const event = new Event('beforeinstallprompt') as MockBeforeInstallPromptEvent;
  event.platforms = ['web'];
  event.userChoice = Promise.resolve({ outcome, platform: 'web' });
  event.prompt = vi.fn().mockResolvedValue(undefined);
  event.preventDefault = vi.fn();
  return event;
}

describe('InstallPrompt', () => {
  let matchMediaMock: ReturnType<typeof vi.fn<[string], MediaQueryList>>;

  const createMatchMediaResult = (matches: boolean, query = ''): MediaQueryList => ({
    matches,
    media: query,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(),
  });

  beforeEach(() => {
    // Clear sessionStorage
    sessionStorage.clear();

    // Mock matchMedia for standalone mode detection
    matchMediaMock = vi
      .fn<[string], MediaQueryList>()
      .mockImplementation((query) => createMatchMediaResult(false, query));
    window.matchMedia = matchMediaMock as unknown as typeof window.matchMedia;
  });

  afterEach(() => {
    vi.restoreAllMocks();
    sessionStorage.clear();
  });

  it('renders nothing by default (no install event)', () => {
    const { container } = render(<InstallPrompt />);
    expect(container.firstChild).toBeNull();
  });

  it('renders nothing when already in standalone mode', () => {
    matchMediaMock.mockImplementation((query) => createMatchMediaResult(true, query));

    const { container } = render(<InstallPrompt />);
    expect(container.firstChild).toBeNull();
  });

  it('renders install prompt when beforeinstallprompt fires', () => {
    render(<InstallPrompt />);

    act(() => {
      const event = createMockInstallEvent();
      window.dispatchEvent(event);
    });

    expect(screen.getByText('Install Gralph Mission Control')).toBeInTheDocument();
    expect(screen.getByText('Add to your home screen for quick access')).toBeInTheDocument();
  });

  it('has proper accessibility attributes', () => {
    render(<InstallPrompt />);

    act(() => {
      window.dispatchEvent(createMockInstallEvent());
    });

    const prompt = screen.getByRole('complementary');
    expect(prompt).toHaveAttribute('aria-label', 'App installation prompt');
  });

  it('shows install and dismiss buttons', () => {
    render(<InstallPrompt />);

    act(() => {
      window.dispatchEvent(createMockInstallEvent());
    });

    expect(screen.getByText('Install')).toBeInTheDocument();
    expect(screen.getByText('Not now')).toBeInTheDocument();
  });

  it('calls prompt when install button is clicked', async () => {
    render(<InstallPrompt />);

    const event = createMockInstallEvent();
    act(() => {
      window.dispatchEvent(event);
    });

    fireEvent.click(screen.getByText('Install'));

    await waitFor(() => {
      expect(event.prompt).toHaveBeenCalled();
    });
  });

  it('hides prompt when user accepts install', async () => {
    render(<InstallPrompt />);

    const event = createMockInstallEvent('accepted');
    act(() => {
      window.dispatchEvent(event);
    });

    expect(screen.getByText('Install Gralph Mission Control')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Install'));

    await waitFor(() => {
      expect(screen.queryByText('Install Gralph Mission Control')).not.toBeInTheDocument();
    });
  });

  it('hides prompt when dismiss button is clicked', () => {
    render(<InstallPrompt />);

    act(() => {
      window.dispatchEvent(createMockInstallEvent());
    });

    expect(screen.getByText('Install Gralph Mission Control')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Not now'));

    expect(screen.queryByText('Install Gralph Mission Control')).not.toBeInTheDocument();
  });

  it('stores dismissal in sessionStorage', () => {
    render(<InstallPrompt />);

    act(() => {
      window.dispatchEvent(createMockInstallEvent());
    });

    fireEvent.click(screen.getByText('Not now'));

    expect(sessionStorage.getItem('pwa-install-dismissed')).toBe('true');
  });

  it('does not show prompt if previously dismissed this session', () => {
    sessionStorage.setItem('pwa-install-dismissed', 'true');

    render(<InstallPrompt />);

    act(() => {
      window.dispatchEvent(createMockInstallEvent());
    });

    expect(screen.queryByText('Install Gralph Mission Control')).not.toBeInTheDocument();
  });

  it('hides prompt when appinstalled event fires', () => {
    render(<InstallPrompt />);

    act(() => {
      window.dispatchEvent(createMockInstallEvent());
    });

    expect(screen.getByText('Install Gralph Mission Control')).toBeInTheDocument();

    act(() => {
      window.dispatchEvent(new Event('appinstalled'));
    });

    expect(screen.queryByText('Install Gralph Mission Control')).not.toBeInTheDocument();
  });

  it('applies custom className', () => {
    render(<InstallPrompt className="custom-class" />);

    act(() => {
      window.dispatchEvent(createMockInstallEvent());
    });

    const prompt = screen.getByRole('complementary');
    expect(prompt).toHaveClass('install-prompt', 'custom-class');
  });

  it('has proper CSS class structure', () => {
    render(<InstallPrompt />);

    act(() => {
      window.dispatchEvent(createMockInstallEvent());
    });

    expect(document.querySelector('.install-prompt')).toBeInTheDocument();
    expect(document.querySelector('.install-prompt__content')).toBeInTheDocument();
    expect(document.querySelector('.install-prompt__icon')).toBeInTheDocument();
    expect(document.querySelector('.install-prompt__text')).toBeInTheDocument();
    expect(document.querySelector('.install-prompt__actions')).toBeInTheDocument();
  });

  it('cleans up event listeners on unmount', () => {
    const addEventListenerSpy = vi.spyOn(window, 'addEventListener');
    const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');

    const { unmount } = render(<InstallPrompt />);

    expect(addEventListenerSpy).toHaveBeenCalledWith('beforeinstallprompt', expect.any(Function));
    expect(addEventListenerSpy).toHaveBeenCalledWith('appinstalled', expect.any(Function));

    unmount();

    expect(removeEventListenerSpy).toHaveBeenCalledWith('beforeinstallprompt', expect.any(Function));
    expect(removeEventListenerSpy).toHaveBeenCalledWith('appinstalled', expect.any(Function));
  });

  it('buttons have correct type attribute', () => {
    render(<InstallPrompt />);

    act(() => {
      window.dispatchEvent(createMockInstallEvent());
    });

    const buttons = screen.getAllByRole('button');
    buttons.forEach((button) => {
      expect(button).toHaveAttribute('type', 'button');
    });
  });
});
