import { ErrorBoundary } from 'react-error-boundary';
import type { ErrorInfo } from 'react';

interface WidgetErrorBoundaryProps {
  children: React.ReactNode;
}

export function WidgetErrorBoundary({ children }: WidgetErrorBoundaryProps) {
  return (
    <ErrorBoundary
      fallback={null}
      onError={(error: Error, info: ErrorInfo) => {
        console.error('[efOfX widget] Runtime error:', error, info);
      }}
    >
      {children}
    </ErrorBoundary>
  );
}
