import { ModpackResponse, ModpackBase, SettingResponse } from '../types/api';
import { authService } from './auth';

const API_BASE = import.meta.env.VITE_API_BASE_URL
  ? `${import.meta.env.VITE_API_BASE_URL}/api/v1`
  : '/api/v1';

class ApiService {
  private handleUnauthorized?: () => void;

  setUnauthorizedHandler(handler: () => void) {
    this.handleUnauthorized = handler;
  }

  private async request<T>(endpoint: string, options?: RequestInit): Promise<T> {
    const authHeaders = authService.getAuthHeaders();

    const response = await fetch(`${API_BASE}${endpoint}`, {
      headers: {
        'Content-Type': 'application/json',
        ...authHeaders,
        ...options?.headers,
      },
      ...options,
    });

    if (response.status === 401) {
      // Handle unauthorized - trigger re-authentication
      if (this.handleUnauthorized) {
        this.handleUnauthorized();
      }
      throw new Error('Unauthorized - please login again');
    }

    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  // Modpacks
  async getModpacks(): Promise<ModpackResponse[]> {
    return this.request<ModpackResponse[]>('/modpacks');
  }

  async getModpack(id: number): Promise<ModpackResponse> {
    return this.request<ModpackResponse>(`/modpacks/${id}`);
  }

  async createModpack(data: ModpackBase): Promise<ModpackResponse> {
    return this.request<ModpackResponse>('/modpacks', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async deleteModpack(id: number): Promise<void> {
    await this.request<void>(`/modpacks/${id}`, {
      method: 'DELETE',
    });
  }

  async updateModpack(id: number, data: ModpackBase): Promise<ModpackResponse> {
    return this.request<ModpackResponse>(`/modpacks/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
  }

  async uploadModpackFiles(id: number, files: FileList): Promise<void> {
    const formData = new FormData();

    // Add all files to FormData
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      // Use the webkitRelativePath to preserve folder structure
      const path = file.webkitRelativePath || file.name;
      formData.append('files', file, path);
    }

    const authHeaders = authService.getAuthHeaders();

    const response = await fetch(`${API_BASE}/modpacks/${id}/files`, {
      method: 'POST',
      headers: {
        // Don't set Content-Type for FormData - browser will set it automatically with boundary
        ...authHeaders,
      },
      body: formData,
    });

    if (response.status === 401) {
      if (this.handleUnauthorized) {
        this.handleUnauthorized();
      }
      throw new Error('Unauthorized - please login again');
    }

    if (!response.ok) {
      throw new Error(`File upload failed: ${response.status} ${response.statusText}`);
    }
  }

  // Minecraft versions and loaders
  async getMinecraftVersions(): Promise<string[]> {
    return this.request<string[]>('/mc-versions');
  }

  async getLoadersForVersion(version: string): Promise<string[]> {
    return this.request<string[]>(`/mc-versions/${version}/loaders`);
  }

  async getLoaderVersions(version: string, loader: string): Promise<string[]> {
    return this.request<string[]>(`/mc-versions/${version}/${loader}`);
  }

  // Settings
  async getSettings(): Promise<SettingResponse[]> {
    return this.request<SettingResponse[]>('/settings');
  }

  async updateSettings(settings: SettingResponse[]): Promise<void> {
    await this.request<void>('/settings', {
      method: 'POST',
      body: JSON.stringify(settings),
    });
  }

  // Build
  async buildModpacks(): Promise<void> {
    await this.request<void>('/modpacks/build', {
      method: 'POST',
    });
  }
}

export const apiService = new ApiService();