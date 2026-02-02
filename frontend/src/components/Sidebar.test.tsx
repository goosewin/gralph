import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Sidebar, type Route, type SidebarSection } from './Sidebar';

describe('Sidebar', () => {
  const mockSections: SidebarSection[] = [
    {
      id: 'dashboard',
      title: 'Dashboard',
      icon: '📊',
      items: [
        { id: 'sessions', label: 'Sessions', route: 'sessions' as Route, icon: '🖥️' },
        { id: 'tasks', label: 'Tasks', route: 'tasks' as Route, icon: '📋', badge: 5 },
      ],
    },
    {
      id: 'settings',
      title: 'Settings',
      icon: '⚙️',
      items: [
        { id: 'preferences', label: 'Preferences', route: 'settings' as Route },
      ],
    },
  ];

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

  it('renders all sections', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    expect(screen.getByText('Dashboard')).toBeInTheDocument();
    expect(screen.getByText('Settings')).toBeInTheDocument();
  });

  it('renders all section items', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    expect(screen.getByText('Sessions')).toBeInTheDocument();
    expect(screen.getByText('Tasks')).toBeInTheDocument();
    expect(screen.getByText('Preferences')).toBeInTheDocument();
  });

  it('displays badges on items', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    expect(screen.getByText('5')).toBeInTheDocument();
  });

  it('highlights active route', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const sessionsButton = screen.getByRole('button', { name: /Sessions/i });
    expect(sessionsButton).toHaveClass('sidebar__item--active');
    expect(sessionsButton).toHaveAttribute('aria-current', 'page');
  });

  it('calls onRouteChange when item is clicked', () => {
    const onRouteChange = vi.fn();
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={onRouteChange}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: /Tasks/i }));

    expect(onRouteChange).toHaveBeenCalledWith('tasks');
  });

  it('calls onClose when item is clicked and onClose is provided', () => {
    const onRouteChange = vi.fn();
    const onClose = vi.fn();
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={onRouteChange}
        onClose={onClose}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: /Tasks/i }));

    expect(onClose).toHaveBeenCalled();
  });

  it('collapses and expands sections', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const dashboardHeader = screen.getByRole('button', { name: /Dashboard/i });

    // Initially expanded
    expect(dashboardHeader).toHaveAttribute('aria-expanded', 'true');

    // Collapse
    fireEvent.click(dashboardHeader);
    expect(dashboardHeader).toHaveAttribute('aria-expanded', 'false');

    // Expand again
    fireEvent.click(dashboardHeader);
    expect(dashboardHeader).toHaveAttribute('aria-expanded', 'true');
  });

  it('persists collapse state to localStorage', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const dashboardHeader = screen.getByRole('button', { name: /Dashboard/i });
    fireEvent.click(dashboardHeader);

    expect(localStorageMock.setItem).toHaveBeenCalledWith(
      'gralph-sidebar-collapse-state',
      expect.stringContaining('"dashboard":true')
    );
  });

  it('loads collapse state from localStorage', () => {
    localStorageMock.setItem(
      'gralph-sidebar-collapse-state',
      JSON.stringify({ dashboard: true })
    );

    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const dashboardHeader = screen.getByRole('button', { name: /Dashboard/i });
    expect(dashboardHeader).toHaveAttribute('aria-expanded', 'false');
  });

  it('applies open class when isOpen is true', () => {
    const { container } = render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
        isOpen={true}
      />
    );

    expect(container.querySelector('.sidebar--open')).toBeInTheDocument();
  });

  it('does not apply open class when isOpen is false', () => {
    const { container } = render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
        isOpen={false}
      />
    );

    expect(container.querySelector('.sidebar--open')).not.toBeInTheDocument();
  });

  it('handles keyboard navigation on items with Enter key', () => {
    const onRouteChange = vi.fn();
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={onRouteChange}
      />
    );

    const tasksButton = screen.getByRole('button', { name: /Tasks/i });
    fireEvent.keyDown(tasksButton, { key: 'Enter' });

    expect(onRouteChange).toHaveBeenCalledWith('tasks');
  });

  it('handles keyboard navigation on items with Space key', () => {
    const onRouteChange = vi.fn();
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={onRouteChange}
      />
    );

    const tasksButton = screen.getByRole('button', { name: /Tasks/i });
    fireEvent.keyDown(tasksButton, { key: ' ' });

    expect(onRouteChange).toHaveBeenCalledWith('tasks');
  });

  it('handles keyboard navigation on section headers with Enter key', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const dashboardHeader = screen.getByRole('button', { name: /Dashboard/i });
    expect(dashboardHeader).toHaveAttribute('aria-expanded', 'true');

    fireEvent.keyDown(dashboardHeader, { key: 'Enter' });
    expect(dashboardHeader).toHaveAttribute('aria-expanded', 'false');
  });

  it('handles keyboard navigation on section headers with Space key', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const dashboardHeader = screen.getByRole('button', { name: /Dashboard/i });
    expect(dashboardHeader).toHaveAttribute('aria-expanded', 'true');

    fireEvent.keyDown(dashboardHeader, { key: ' ' });
    expect(dashboardHeader).toHaveAttribute('aria-expanded', 'false');
  });

  it('has proper ARIA attributes for accessibility', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const sidebar = screen.getByRole('navigation');
    expect(sidebar.closest('aside')).toHaveAttribute('aria-label', 'Main navigation');

    // Check section header accessibility
    const dashboardHeader = screen.getByRole('button', { name: /Dashboard/i });
    expect(dashboardHeader).toHaveAttribute('aria-expanded');
    expect(dashboardHeader).toHaveAttribute('aria-controls');
  });

  it('applies has-active class to section containing active route', () => {
    const { container } = render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const sections = container.querySelectorAll('.sidebar__section');
    expect(sections[0]).toHaveClass('sidebar__section--has-active');
    expect(sections[1]).not.toHaveClass('sidebar__section--has-active');
  });

  it('renders items without icons', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="settings"
        onRouteChange={() => {}}
      />
    );

    // Preferences item has no icon
    const preferencesButton = screen.getByRole('button', { name: /Preferences/i });
    expect(preferencesButton.querySelector('.sidebar__item-icon')).not.toBeInTheDocument();
  });

  it('renders items with icons', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const sessionsButton = screen.getByRole('button', { name: /Sessions/i });
    expect(sessionsButton.querySelector('.sidebar__item-icon')).toBeInTheDocument();
  });

  it('does not call onRouteChange on other key presses', () => {
    const onRouteChange = vi.fn();
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={onRouteChange}
      />
    );

    const tasksButton = screen.getByRole('button', { name: /Tasks/i });
    fireEvent.keyDown(tasksButton, { key: 'Tab' });
    fireEvent.keyDown(tasksButton, { key: 'Escape' });
    fireEvent.keyDown(tasksButton, { key: 'ArrowDown' });

    expect(onRouteChange).not.toHaveBeenCalled();
  });

  it('does not toggle section on other key presses', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const dashboardHeader = screen.getByRole('button', { name: /Dashboard/i });
    expect(dashboardHeader).toHaveAttribute('aria-expanded', 'true');

    fireEvent.keyDown(dashboardHeader, { key: 'Tab' });
    fireEvent.keyDown(dashboardHeader, { key: 'Escape' });

    expect(dashboardHeader).toHaveAttribute('aria-expanded', 'true');
  });

  it('renders badge with aria-label', () => {
    render(
      <Sidebar
        sections={mockSections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    const badge = screen.getByText('5');
    expect(badge).toHaveAttribute('aria-label', '5 items');
  });

  it('handles empty sections gracefully', () => {
    const emptySections: SidebarSection[] = [];

    render(
      <Sidebar
        sections={emptySections}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    // Should render the navigation container without throwing
    expect(screen.getByRole('navigation')).toBeInTheDocument();
  });

  it('handles sections with no items', () => {
    const sectionsWithEmpty: SidebarSection[] = [
      {
        id: 'empty',
        title: 'Empty Section',
        icon: '📭',
        items: [],
      },
    ];

    render(
      <Sidebar
        sections={sectionsWithEmpty}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    expect(screen.getByText('Empty Section')).toBeInTheDocument();
  });

  it('renders items with numeric badges', () => {
    const sectionsWithNumericBadge: SidebarSection[] = [
      {
        id: 'test',
        title: 'Test',
        icon: '🧪',
        items: [
          { id: 'item1', label: 'Item', route: 'sessions' as Route, badge: 42 },
        ],
      },
    ];

    render(
      <Sidebar
        sections={sectionsWithNumericBadge}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    expect(screen.getByText('42')).toBeInTheDocument();
  });

  it('renders items with string badges', () => {
    const sectionsWithStringBadge: SidebarSection[] = [
      {
        id: 'test',
        title: 'Test',
        icon: '🧪',
        items: [
          { id: 'item1', label: 'Item', route: 'sessions' as Route, badge: 'new' },
        ],
      },
    ];

    render(
      <Sidebar
        sections={sectionsWithStringBadge}
        activeRoute="sessions"
        onRouteChange={() => {}}
      />
    );

    expect(screen.getByText('new')).toBeInTheDocument();
  });
});
