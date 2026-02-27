import { WidgetProvider } from './context/WidgetContext';
import { FloatingButton } from './components/FloatingButton';
import { ChatPanel } from './components/ChatPanel';
import type { WidgetConfig, BrandingConfig } from './types/widget';

interface AppProps {
  config: WidgetConfig;
  branding?: BrandingConfig | null;
}

/**
 * App — Root widget component
 *
 * Wraps everything in WidgetProvider for shared state access.
 * Renders FloatingButton + ChatPanel for floating mode, or just ChatPanel for inline mode.
 */
function App({ config, branding }: AppProps) {
  return (
    <WidgetProvider config={config} branding={branding}>
      {config.mode === 'floating' ? (
        <>
          <FloatingButton />
          <ChatPanel />
        </>
      ) : (
        <ChatPanel />
      )}
    </WidgetProvider>
  );
}

export default App;
