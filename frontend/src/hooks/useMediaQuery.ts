import { useCallback, useEffect, useState } from 'react';

/**
 * Hook to detect if a media query matches.
 * @param query - CSS media query string (e.g., '(min-width: 768px)')
 * @returns boolean indicating if the media query matches
 */
export function useMediaQuery(query: string): boolean {
  const getMatches = useCallback((query: string): boolean => {
    // Prevent SSR issues
    if (typeof window !== 'undefined') {
      return window.matchMedia(query).matches;
    }
    return false;
  }, []);

  const [matches, setMatches] = useState<boolean>(() => getMatches(query));

  useEffect(() => {
    const mediaQuery = window.matchMedia(query);

    // Update state initially
    setMatches(mediaQuery.matches);

    // Handler for changes
    const handleChange = (event: MediaQueryListEvent) => {
      setMatches(event.matches);
    };

    // Modern browsers
    if (mediaQuery.addEventListener) {
      mediaQuery.addEventListener('change', handleChange);
      return () => mediaQuery.removeEventListener('change', handleChange);
    }
    // Legacy browsers (Safari < 14)
    else {
      mediaQuery.addListener(handleChange);
      return () => mediaQuery.removeListener(handleChange);
    }
  }, [query]);

  return matches;
}

// Breakpoint constants matching CSS
export const BREAKPOINTS = {
  sm: '640px',
  md: '768px',
  lg: '1024px',
} as const;

/**
 * Hook to detect common breakpoints.
 * Returns an object with boolean values for each breakpoint.
 */
export function useBreakpoints() {
  const isSm = useMediaQuery(`(min-width: ${BREAKPOINTS.sm})`);
  const isMd = useMediaQuery(`(min-width: ${BREAKPOINTS.md})`);
  const isLg = useMediaQuery(`(min-width: ${BREAKPOINTS.lg})`);
  const isTouchDevice = useMediaQuery('(hover: none) and (pointer: coarse)');

  return {
    /** >= 640px */
    isSm,
    /** >= 768px */
    isMd,
    /** >= 1024px */
    isLg,
    /** Touch device (no hover, coarse pointer) */
    isTouchDevice,
    /** Mobile (<768px) */
    isMobile: !isMd,
    /** Tablet (768px - 1023px) */
    isTablet: isMd && !isLg,
    /** Desktop (>=1024px) */
    isDesktop: isLg,
  };
}
