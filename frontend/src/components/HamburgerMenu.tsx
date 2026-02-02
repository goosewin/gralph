export interface HamburgerMenuProps {
  isOpen: boolean;
  onToggle: () => void;
  'aria-controls'?: string;
}

export function HamburgerMenu({
  isOpen,
  onToggle,
  'aria-controls': ariaControls,
}: HamburgerMenuProps) {
  return (
    <button
      className={`hamburger-menu ${isOpen ? 'hamburger-menu--open' : ''}`}
      onClick={onToggle}
      aria-expanded={isOpen}
      aria-controls={ariaControls}
      aria-label={isOpen ? 'Close navigation menu' : 'Open navigation menu'}
      type="button"
    >
      <span className="hamburger-menu__line" aria-hidden="true" />
      <span className="hamburger-menu__line" aria-hidden="true" />
      <span className="hamburger-menu__line" aria-hidden="true" />
    </button>
  );
}
