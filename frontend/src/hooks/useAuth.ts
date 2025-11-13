import { useState, useEffect } from 'react';
import { authService } from '../services/auth';
import { TokenRequest } from '../types/auth';

export function useAuth() {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Check if user is already authenticated
    setIsAuthenticated(authService.isAuthenticated());
  }, []);

  const login = async (tokenRequest: TokenRequest) => {
    try {
      setLoading(true);
      setError(null);

      await authService.login(tokenRequest);
      setIsAuthenticated(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
      setIsAuthenticated(false);
    } finally {
      setLoading(false);
    }
  };

  const logout = () => {
    authService.logout();
    setIsAuthenticated(false);
    setError(null);
  };

  return {
    isAuthenticated,
    loading,
    error,
    login,
    logout,
  };
}