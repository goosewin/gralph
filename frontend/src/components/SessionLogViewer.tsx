import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from 'react';

export interface SessionLogViewerProps {
  /** Log lines to display */
  logs: string[];
  /** Total number of lines (may differ from logs.length if paginated) */
  totalLines: number;
  /** Session name for display */
  sessionName?: string;
  /** Path to the log file */
  logFile?: string | null;
  /** Whether the component is loading */
  loading?: boolean;
  /** Error message to display */
  error?: string | null;
  /** Callback to refresh logs */
  onRefresh?: () => void;
  /** Callback to toggle raw/processed logs */
  onToggleRaw?: (raw: boolean) => void;
  /** Whether showing raw logs */
  isRaw?: boolean;
}

/** Classify a log line for syntax highlighting */
type LogLineType = 'error' | 'warning' | 'success' | 'info' | 'default';

function classifyLine(line: string): LogLineType {
  const lower = line.toLowerCase();
  if (
    lower.includes('error') ||
    lower.includes('failed') ||
    lower.includes('failure') ||
    lower.startsWith('error:')
  ) {
    return 'error';
  }
  if (
    lower.includes('warning') ||
    lower.includes('warn') ||
    lower.startsWith('warning:')
  ) {
    return 'warning';
  }
  if (
    lower.includes('success') ||
    lower.includes('completed') ||
    lower.includes('passed') ||
    lower.includes('done')
  ) {
    return 'success';
  }
  if (
    lower.startsWith('info:') ||
    lower.startsWith('[info]') ||
    lower.includes('iteration')
  ) {
    return 'info';
  }
  return 'default';
}

/** Highlight search term in text */
function highlightText(
  text: string,
  searchTerm: string,
  isActive: boolean
): React.ReactNode {
  if (!searchTerm) {
    return text;
  }

  const regex = new RegExp(`(${searchTerm.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
  const parts = text.split(regex);

  return parts.map((part, index) => {
    if (part.toLowerCase() === searchTerm.toLowerCase()) {
      return (
        <mark
          key={index}
          className={`log-viewer__highlight ${isActive ? 'log-viewer__highlight--active' : ''}`}
        >
          {part}
        </mark>
      );
    }
    return part;
  });
}

/** Virtual list item height in pixels */
const LINE_HEIGHT = 20;
/** Number of lines to render above/below viewport for smooth scrolling */
const OVERSCAN = 20;

export function SessionLogViewer({
  logs,
  totalLines,
  sessionName,
  logFile,
  loading = false,
  error,
  onRefresh,
  onToggleRaw,
  isRaw = false,
}: SessionLogViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [searchResults, setSearchResults] = useState<number[]>([]);
  const [currentSearchIndex, setCurrentSearchIndex] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const [containerHeight, setContainerHeight] = useState(400);
  const prevLogsLengthRef = useRef(logs.length);

  // Calculate visible range for virtualization
  const visibleRange = useMemo(() => {
    const startIndex = Math.max(0, Math.floor(scrollTop / LINE_HEIGHT) - OVERSCAN);
    const endIndex = Math.min(
      logs.length,
      Math.ceil((scrollTop + containerHeight) / LINE_HEIGHT) + OVERSCAN
    );
    return { startIndex, endIndex };
  }, [scrollTop, containerHeight, logs.length]);

  // Visible logs (virtualized)
  const visibleLogs = useMemo(() => {
    return logs.slice(visibleRange.startIndex, visibleRange.endIndex).map((line, index) => ({
      line,
      lineNumber: visibleRange.startIndex + index + 1,
      globalIndex: visibleRange.startIndex + index,
    }));
  }, [logs, visibleRange]);

  // Search functionality
  useEffect(() => {
    if (!searchTerm) {
      setSearchResults([]);
      setCurrentSearchIndex(0);
      return;
    }

    const lowerSearch = searchTerm.toLowerCase();
    const results: number[] = [];
    logs.forEach((line, index) => {
      if (line.toLowerCase().includes(lowerSearch)) {
        results.push(index);
      }
    });
    setSearchResults(results);
    setCurrentSearchIndex(0);
  }, [searchTerm, logs]);

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (autoScroll && logs.length > prevLogsLengthRef.current && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
    prevLogsLengthRef.current = logs.length;
  }, [logs.length, autoScroll]);

  // Update container height on resize
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerHeight(entry.contentRect.height);
      }
    });

    resizeObserver.observe(container);
    setContainerHeight(container.clientHeight);

    return () => resizeObserver.disconnect();
  }, []);

  // Handle scroll events
  const handleScroll = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;

    setScrollTop(container.scrollTop);

    // Disable auto-scroll if user scrolls up
    const isAtBottom =
      container.scrollHeight - container.scrollTop - container.clientHeight < 50;
    if (!isAtBottom && autoScroll) {
      setAutoScroll(false);
    }
  }, [autoScroll]);

  // Navigate to search result
  const goToSearchResult = useCallback(
    (index: number) => {
      if (searchResults.length === 0) return;
      const resultIndex = searchResults[index];
      if (resultIndex === undefined) return;

      setCurrentSearchIndex(index);

      // Scroll to the line
      if (containerRef.current) {
        const targetScrollTop = resultIndex * LINE_HEIGHT - containerHeight / 2 + LINE_HEIGHT / 2;
        containerRef.current.scrollTop = Math.max(0, targetScrollTop);
      }
    },
    [searchResults, containerHeight]
  );

  const handlePrevResult = useCallback(() => {
    if (searchResults.length === 0) return;
    const newIndex = (currentSearchIndex - 1 + searchResults.length) % searchResults.length;
    goToSearchResult(newIndex);
  }, [searchResults.length, currentSearchIndex, goToSearchResult]);

  const handleNextResult = useCallback(() => {
    if (searchResults.length === 0) return;
    const newIndex = (currentSearchIndex + 1) % searchResults.length;
    goToSearchResult(newIndex);
  }, [searchResults.length, currentSearchIndex, goToSearchResult]);

  const handleSearchKeyDown = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        if (e.shiftKey) {
          handlePrevResult();
        } else {
          handleNextResult();
        }
      } else if (e.key === 'Escape') {
        setSearchTerm('');
      }
    },
    [handlePrevResult, handleNextResult]
  );

  const toggleAutoScroll = useCallback(() => {
    setAutoScroll((prev) => {
      if (!prev && containerRef.current) {
        // When enabling, scroll to bottom
        containerRef.current.scrollTop = containerRef.current.scrollHeight;
      }
      return !prev;
    });
  }, []);

  const handleScrollToTop = useCallback(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = 0;
      setAutoScroll(false);
    }
  }, []);

  const handleScrollToBottom = useCallback(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
      setAutoScroll(true);
    }
  }, []);

  const totalHeight = logs.length * LINE_HEIGHT;
  const offsetY = visibleRange.startIndex * LINE_HEIGHT;

  return (
    <div className="log-viewer">
      <div className="log-viewer__header">
        <div className="log-viewer__title">
          <h2>Session Logs{sessionName ? `: ${sessionName}` : ''}</h2>
          {logFile && (
            <span className="log-viewer__file" title={logFile}>
              {logFile.split('/').pop()}
            </span>
          )}
        </div>
        <div className="log-viewer__stats">
          <span className="log-viewer__line-count">
            {totalLines.toLocaleString()} lines
          </span>
        </div>
      </div>

      <div className="log-viewer__toolbar">
        <div className="log-viewer__search">
          <input
            type="text"
            className="log-viewer__search-input"
            placeholder="Search logs... (Enter for next, Shift+Enter for prev)"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            onKeyDown={handleSearchKeyDown}
            aria-label="Search logs"
          />
          {searchTerm && (
            <span className="log-viewer__search-results">
              {searchResults.length > 0
                ? `${currentSearchIndex + 1} / ${searchResults.length}`
                : 'No results'}
            </span>
          )}
          {searchResults.length > 0 && (
            <div className="log-viewer__search-nav">
              <button
                type="button"
                className="log-viewer__search-nav-btn"
                onClick={handlePrevResult}
                aria-label="Previous result"
              >
                &#9650;
              </button>
              <button
                type="button"
                className="log-viewer__search-nav-btn"
                onClick={handleNextResult}
                aria-label="Next result"
              >
                &#9660;
              </button>
            </div>
          )}
        </div>

        <div className="log-viewer__controls">
          {onToggleRaw && (
            <button
              type="button"
              className={`log-viewer__control-btn ${isRaw ? 'log-viewer__control-btn--active' : ''}`}
              onClick={() => onToggleRaw(!isRaw)}
              aria-pressed={isRaw}
            >
              Raw
            </button>
          )}
          <button
            type="button"
            className={`log-viewer__control-btn ${autoScroll ? 'log-viewer__control-btn--active' : ''}`}
            onClick={toggleAutoScroll}
            aria-pressed={autoScroll}
            title="Auto-scroll to new content"
          >
            Auto-scroll
          </button>
          <button
            type="button"
            className="log-viewer__control-btn"
            onClick={handleScrollToTop}
            title="Scroll to top"
          >
            Top
          </button>
          <button
            type="button"
            className="log-viewer__control-btn"
            onClick={handleScrollToBottom}
            title="Scroll to bottom"
          >
            Bottom
          </button>
          {onRefresh && (
            <button
              type="button"
              className="log-viewer__control-btn log-viewer__control-btn--primary"
              onClick={onRefresh}
              disabled={loading}
            >
              {loading ? 'Loading...' : 'Refresh'}
            </button>
          )}
        </div>
      </div>

      {error && (
        <div className="log-viewer__error" role="alert">
          {error}
        </div>
      )}

      <div
        ref={containerRef}
        className="log-viewer__container"
        onScroll={handleScroll}
        role="log"
        aria-live="polite"
        aria-label="Session log output"
        tabIndex={0}
      >
        {logs.length === 0 && !loading && (
          <div className="log-viewer__empty">
            No log content available
          </div>
        )}
        {logs.length > 0 && (
          <div
            className="log-viewer__content"
            style={{ height: totalHeight, position: 'relative' }}
          >
            <div
              style={{
                position: 'absolute',
                top: offsetY,
                left: 0,
                right: 0,
              }}
            >
              {visibleLogs.map(({ line, lineNumber, globalIndex }) => {
                const lineType = classifyLine(line);
                const isSearchMatch =
                  searchTerm && line.toLowerCase().includes(searchTerm.toLowerCase());
                const isActiveMatch =
                  searchResults.length > 0 &&
                  searchResults[currentSearchIndex] === globalIndex;

                return (
                  <div
                    key={globalIndex}
                    className={`log-viewer__line log-viewer__line--${lineType} ${
                      isSearchMatch ? 'log-viewer__line--match' : ''
                    } ${isActiveMatch ? 'log-viewer__line--active-match' : ''}`}
                    style={{ height: LINE_HEIGHT }}
                    data-line-number={lineNumber}
                  >
                    <span className="log-viewer__line-number">{lineNumber}</span>
                    <span className="log-viewer__line-content">
                      {highlightText(line, searchTerm, isActiveMatch)}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        )}
        {loading && logs.length === 0 && (
          <div className="log-viewer__loading">Loading logs...</div>
        )}
      </div>

      {autoScroll && (
        <div className="log-viewer__auto-scroll-indicator" aria-hidden="true">
          Auto-scrolling enabled
        </div>
      )}
    </div>
  );
}
