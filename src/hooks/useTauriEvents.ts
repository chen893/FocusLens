import { useEffect, useRef } from "react";
import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * A safe hook to listen to Tauri events with automatic cleanup.
 */
export function useTauriEvent<T>(
  eventName: string,
  callback: EventCallback<T>,
  deps: React.DependencyList = []
) {
  const callbackRef = useRef(callback);

  // Keep callback up to date without re-triggering the listener
  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let disposed = false;

    const setupListener = async () => {
      const cleanup = await listen<T>(eventName, (event) => {
        if (!disposed) {
          callbackRef.current(event);
        }
      });

      if (disposed) {
        cleanup();
      } else {
        unlisten = cleanup;
      }
    };

    void setupListener();

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventName, ...deps]);
}
