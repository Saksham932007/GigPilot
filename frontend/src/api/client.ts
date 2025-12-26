import axios from 'axios';

/**
 * API client for GigPilot backend
 * 
 * Handles authentication and base URL configuration.
 */

const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:3000';

export const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Add JWT token to requests
api.interceptors.request.use((config) => {
  const token = getAuthToken();
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// Handle 401 errors (unauthorized)
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      // Handle logout/redirect to login
      clearAuthToken();
      // Redirect to login page
      if (typeof window !== 'undefined') {
        window.location.href = '/login';
      }
    }
    return Promise.reject(error);
  }
);

function getAuthToken(): string | null {
  if (typeof window !== 'undefined') {
    return localStorage.getItem('gigpilot_auth_token');
  }
  return null;
}

function clearAuthToken(): void {
  if (typeof window !== 'undefined') {
    localStorage.removeItem('gigpilot_auth_token');
  }
}

