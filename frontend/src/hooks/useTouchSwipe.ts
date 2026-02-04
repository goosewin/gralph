import { useCallback, useRef, useState } from 'react';

export type SwipeDirection = 'left' | 'right' | 'up' | 'down' | null;

export interface TouchSwipeConfig {
  /** Minimum distance in pixels to trigger a swipe (default: 50) */
  minSwipeDistance?: number;
  /** Maximum time in ms for a swipe gesture (default: 300) */
  maxSwipeTime?: number;
  /** Callback when swipe is detected */
  onSwipe?: (direction: SwipeDirection) => void;
  /** Callback when swiping left */
  onSwipeLeft?: () => void;
  /** Callback when swiping right */
  onSwipeRight?: () => void;
  /** Whether swipe detection is enabled (default: true) */
  enabled?: boolean;
}

export interface TouchSwipeResult {
  /** Props to spread on the target element */
  handlers: {
    onTouchStart: (e: React.TouchEvent) => void;
    onTouchMove: (e: React.TouchEvent) => void;
    onTouchEnd: (e: React.TouchEvent) => void;
  };
  /** Current swipe state */
  swiping: boolean;
  /** Current swipe direction (null if not swiping) */
  direction: SwipeDirection;
  /** Current swipe distance in pixels */
  distance: number;
}

interface TouchState {
  startX: number;
  startY: number;
  startTime: number;
  currentX: number;
  currentY: number;
}

export function useTouchSwipe({
  minSwipeDistance = 50,
  maxSwipeTime = 300,
  onSwipe,
  onSwipeLeft,
  onSwipeRight,
  enabled = true,
}: TouchSwipeConfig = {}): TouchSwipeResult {
  const [swiping, setSwiping] = useState(false);
  const [direction, setDirection] = useState<SwipeDirection>(null);
  const [distance, setDistance] = useState(0);
  const touchState = useRef<TouchState | null>(null);

  const handleTouchStart = useCallback(
    (e: React.TouchEvent) => {
      if (!enabled) return;

      const touch = e.touches[0];
      touchState.current = {
        startX: touch.clientX,
        startY: touch.clientY,
        startTime: Date.now(),
        currentX: touch.clientX,
        currentY: touch.clientY,
      };
      setSwiping(true);
      setDirection(null);
      setDistance(0);
    },
    [enabled]
  );

  const handleTouchMove = useCallback(
    (e: React.TouchEvent) => {
      if (!enabled || !touchState.current) return;

      const touch = e.touches[0];
      touchState.current.currentX = touch.clientX;
      touchState.current.currentY = touch.clientY;

      const deltaX = touch.clientX - touchState.current.startX;
      const deltaY = touch.clientY - touchState.current.startY;
      const absX = Math.abs(deltaX);
      const absY = Math.abs(deltaY);

      // Determine primary direction
      if (absX > absY && absX > 10) {
        setDirection(deltaX > 0 ? 'right' : 'left');
        setDistance(absX);
      } else if (absY > absX && absY > 10) {
        setDirection(deltaY > 0 ? 'down' : 'up');
        setDistance(absY);
      }
    },
    [enabled]
  );

  const handleTouchEnd = useCallback(
    (event: React.TouchEvent) => {
      void event;
      if (!enabled || !touchState.current) {
        setSwiping(false);
        return;
      }

      const { startX, startY, startTime, currentX, currentY } = touchState.current;
      const deltaX = currentX - startX;
      const deltaY = currentY - startY;
      const absX = Math.abs(deltaX);
      const absY = Math.abs(deltaY);
      const elapsedTime = Date.now() - startTime;

      // Check if it's a valid swipe
      if (elapsedTime <= maxSwipeTime) {
        // Horizontal swipe
        if (absX > absY && absX >= minSwipeDistance) {
          const swipeDir: SwipeDirection = deltaX > 0 ? 'right' : 'left';
          onSwipe?.(swipeDir);
          if (swipeDir === 'left') {
            onSwipeLeft?.();
          } else if (swipeDir === 'right') {
            onSwipeRight?.();
          }
        }
        // Vertical swipe (for future use)
        else if (absY > absX && absY >= minSwipeDistance) {
          const swipeDir: SwipeDirection = deltaY > 0 ? 'down' : 'up';
          onSwipe?.(swipeDir);
        }
      }

      touchState.current = null;
      setSwiping(false);
      setDirection(null);
      setDistance(0);
    },
    [enabled, minSwipeDistance, maxSwipeTime, onSwipe, onSwipeLeft, onSwipeRight]
  );

  return {
    handlers: {
      onTouchStart: handleTouchStart,
      onTouchMove: handleTouchMove,
      onTouchEnd: handleTouchEnd,
    },
    swiping,
    direction,
    distance,
  };
}
