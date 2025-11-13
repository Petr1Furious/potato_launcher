import { useEffect, useRef } from 'react';
import { webSocketService } from '../services/websocket';
import { authService } from '../services/auth';

interface UseWebSocketOptions {
  onModpackChange?: (data: any) => void;
  onNotification?: (data: any) => void;
  enabled?: boolean;
}

export function useWebSocket({ onModpackChange, onNotification, enabled = true }: UseWebSocketOptions = {}) {
  const handlersRef = useRef({ onModpackChange, onNotification });

  // Update handlers ref when they change
  useEffect(() => {
    handlersRef.current = { onModpackChange, onNotification };
  }, [onModpackChange, onNotification]);

  useEffect(() => {
    if (!enabled) return;

    const token = authService.getToken();
    if (!token) return;

    // Connect to WebSocket
    webSocketService.connect(token).catch(error => {
      console.error('Failed to connect to WebSocket:', error);
    });

    // Set up event listeners
    const handleModpackChange = (event: CustomEvent) => {
      if (handlersRef.current.onModpackChange) {
        handlersRef.current.onModpackChange(event.detail);
      }
    };

    const handleNotification = (event: CustomEvent) => {
      if (handlersRef.current.onNotification) {
        handlersRef.current.onNotification(event.detail);
      }
    };

    window.addEventListener('modpack_change', handleModpackChange as EventListener);
    window.addEventListener('notification', handleNotification as EventListener);

    // Cleanup
    return () => {
      window.removeEventListener('modpack_change', handleModpackChange as EventListener);
      window.removeEventListener('notification', handleNotification as EventListener);
      webSocketService.disconnect();
    };
  }, [enabled]);

  return {
    isConnected: webSocketService.isConnected(),
  };
}