import { ErrorBoundary as ReactErrorBoundary } from 'react-error-boundary';
import type { ErrorInfo, ReactNode } from 'react';

export interface ErrorBoundaryProps {
  children: ReactNode;
}

/**
 * ErrorBoundary — Wraps children in a React error boundary.
 *
 * Silently catches runtime errors (fallback=null) and logs to console.
 * Prevents uncaught React render errors from crashing the host page.
 */
export function ErrorBoundary({ children }: ErrorBoundaryProps) {
  return (
    <ReactErrorBoundary
      fallback={null}
      onError={(error: unknown, info: ErrorInfo) => {
        console.error('[efOfX] Runtime error:', error, info);
      }}
    >
      {children}
    </ReactErrorBoundary>
  );
}

export default ErrorBoundary;
