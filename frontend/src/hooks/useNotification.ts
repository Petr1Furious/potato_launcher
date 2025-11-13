import { useState, useCallback } from 'react';
import { NotificationType } from '../components/Notification';

interface NotificationState {
  isVisible: boolean;
  type: NotificationType;
  message: string;
}

export function useNotification() {
  const [notification, setNotification] = useState<NotificationState>({
    isVisible: false,
    type: 'info',
    message: ''
  });

  const showNotification = useCallback((type: NotificationType, message: string) => {
    setNotification({
      isVisible: true,
      type,
      message
    });
  }, []);

  const hideNotification = useCallback(() => {
    setNotification(prev => ({
      ...prev,
      isVisible: false
    }));
  }, []);

  const showSuccess = useCallback((message: string) => {
    showNotification('success', message);
  }, [showNotification]);

  const showError = useCallback((message: string) => {
    showNotification('error', message);
  }, [showNotification]);

  const showWarning = useCallback((message: string) => {
    showNotification('warning', message);
  }, [showNotification]);

  const showInfo = useCallback((message: string) => {
    showNotification('info', message);
  }, [showNotification]);

  return {
    notification,
    showNotification,
    hideNotification,
    showSuccess,
    showError,
    showWarning,
    showInfo
  };
}