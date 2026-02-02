import { useCallback, useId, useMemo } from 'react';
import { useLocalStorage } from '../hooks/useLocalStorage';

export type Route = 'sessions' | 'tasks' | 'logs' | 'orchestration' | 'settings' | 'user-settings' | 'admin-users';

export interface SidebarSection {
  id: string;
  title: string;
  icon: string;
  items: SidebarItem[];
}

export interface SidebarItem {
  id: string;
  label: string;
  route: Route;
  icon?: string;
  badge?: string | number;
}

export interface SidebarProps {
  sections: SidebarSection[];
  activeRoute: Route;
  onRouteChange: (route: Route) => void;
  isOpen?: boolean;
  onClose?: () => void;
}

const COLLAPSE_STATE_KEY = 'gralph-sidebar-collapse-state';

interface CollapseState {
  [sectionId: string]: boolean;
}

export function Sidebar({
  sections,
  activeRoute,
  onRouteChange,
  isOpen = true,
  onClose,
}: SidebarProps) {
  const [collapseState, setCollapseState] = useLocalStorage<CollapseState>(
    COLLAPSE_STATE_KEY,
    {}
  );

  const toggleSection = useCallback(
    (sectionId: string) => {
      setCollapseState((prev) => ({
        ...prev,
        [sectionId]: !prev[sectionId],
      }));
    },
    [setCollapseState]
  );

  const handleItemClick = useCallback(
    (route: Route) => {
      onRouteChange(route);
      onClose?.();
    },
    [onRouteChange, onClose]
  );

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent, route: Route) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        handleItemClick(route);
      }
    },
    [handleItemClick]
  );

  const handleSectionKeyDown = useCallback(
    (event: React.KeyboardEvent, sectionId: string) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        toggleSection(sectionId);
      }
    },
    [toggleSection]
  );

  return (
    <aside
      className={`sidebar ${isOpen ? 'sidebar--open' : ''}`}
      aria-label="Main navigation"
    >
      <nav className="sidebar__nav" role="navigation">
        {sections.map((section) => (
          <SidebarSectionComponent
            key={section.id}
            section={section}
            isCollapsed={collapseState[section.id] ?? false}
            activeRoute={activeRoute}
            onToggle={() => toggleSection(section.id)}
            onItemClick={handleItemClick}
            onKeyDown={handleKeyDown}
            onSectionKeyDown={handleSectionKeyDown}
          />
        ))}
      </nav>
    </aside>
  );
}

interface SidebarSectionComponentProps {
  section: SidebarSection;
  isCollapsed: boolean;
  activeRoute: Route;
  onToggle: () => void;
  onItemClick: (route: Route) => void;
  onKeyDown: (event: React.KeyboardEvent, route: Route) => void;
  onSectionKeyDown: (event: React.KeyboardEvent, sectionId: string) => void;
}

function SidebarSectionComponent({
  section,
  isCollapsed,
  activeRoute,
  onToggle,
  onItemClick,
  onKeyDown,
  onSectionKeyDown,
}: SidebarSectionComponentProps) {
  const contentId = useId();
  const headerId = useId();

  const hasActiveItem = useMemo(
    () => section.items.some((item) => item.route === activeRoute),
    [section.items, activeRoute]
  );

  return (
    <div
      className={`sidebar__section ${hasActiveItem ? 'sidebar__section--has-active' : ''}`}
    >
      <button
        id={headerId}
        className="sidebar__section-header"
        onClick={onToggle}
        onKeyDown={(e) => onSectionKeyDown(e, section.id)}
        aria-expanded={!isCollapsed}
        aria-controls={contentId}
        type="button"
      >
        <span className="sidebar__section-icon" aria-hidden="true">
          {section.icon}
        </span>
        <span className="sidebar__section-title">{section.title}</span>
        <span
          className={`sidebar__section-chevron ${isCollapsed ? 'sidebar__section-chevron--collapsed' : ''}`}
          aria-hidden="true"
        >
          <ChevronIcon />
        </span>
      </button>
      <ul
        id={contentId}
        className={`sidebar__section-content ${isCollapsed ? 'sidebar__section-content--collapsed' : ''}`}
        role="list"
        aria-labelledby={headerId}
      >
        {section.items.map((item) => (
          <li key={item.id} role="listitem">
            <button
              className={`sidebar__item ${activeRoute === item.route ? 'sidebar__item--active' : ''}`}
              onClick={() => onItemClick(item.route)}
              onKeyDown={(e) => onKeyDown(e, item.route)}
              aria-current={activeRoute === item.route ? 'page' : undefined}
              type="button"
            >
              {item.icon && (
                <span className="sidebar__item-icon" aria-hidden="true">
                  {item.icon}
                </span>
              )}
              <span className="sidebar__item-label">{item.label}</span>
              {item.badge !== undefined && (
                <span className="sidebar__item-badge" aria-label={`${item.badge} items`}>
                  {item.badge}
                </span>
              )}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

function ChevronIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <path
        d="M4 6L8 10L12 6"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
