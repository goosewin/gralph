import type { Theme } from '../hooks/useTheme';

export interface ThemeToggleProps {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
}

const THEME_OPTIONS: { value: Theme; label: string; icon: string }[] = [
  { value: 'light', label: 'Light', icon: '☀️' },
  { value: 'dark', label: 'Dark', icon: '🌙' },
  { value: 'system', label: 'System', icon: '💻' },
];

export function ThemeToggle({ theme, onThemeChange }: ThemeToggleProps) {
  return (
    <div className="theme-toggle" role="group" aria-label="Theme selection">
      {THEME_OPTIONS.map((option) => (
        <button
          key={option.value}
          className={`theme-toggle__button ${theme === option.value ? 'theme-toggle__button--active' : ''}`}
          onClick={() => onThemeChange(option.value)}
          aria-pressed={theme === option.value}
          title={option.label}
        >
          <span className="theme-toggle__icon" aria-hidden="true">
            {option.icon}
          </span>
          <span className="sr-only">{option.label} theme</span>
        </button>
      ))}
    </div>
  );
}
