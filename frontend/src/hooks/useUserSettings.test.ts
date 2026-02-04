import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useUserSettings } from './useUserSettings';

// Mock fetch globally
const mockFetch = vi.fn();
const originalFetch = globalThis.fetch;

describe('useUserSettings', () => {
  const mockProfile = {
    user: {
      id: 'user-1',
      email: 'test@example.com',
      name: 'Test User',
      role: 'developer' as const,
      created_at: '2024-01-01T00:00:00Z',
    },
  };

  const mockLinkedAccounts = {
    accounts: [
      {
        provider: 'github' as const,
        provider_user_id: '12345',
        email: 'test@github.com',
        username: 'testuser',
        linked_at: 1704067200,
      },
    ],
  };

  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.fetch = mockFetch as typeof fetch;
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({}),
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    globalThis.fetch = originalFetch;
  });

  describe('initial state', () => {
    it('starts with null profile', () => {
      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));
      expect(result.current.profile).toBeNull();
    });

    it('starts with empty linked accounts', () => {
      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));
      expect(result.current.linkedAccounts).toEqual([]);
    });

    it('starts with loading false', () => {
      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));
      expect(result.current.loading).toBe(false);
    });

    it('starts with null error', () => {
      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));
      expect(result.current.error).toBeNull();
    });
  });

  describe('fetchProfile', () => {
    it('fetches profile from API', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockProfile),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchProfile();
      });

      expect(mockFetch).toHaveBeenCalledWith(
        '/auth/me',
        expect.objectContaining({
          method: 'GET',
          headers: expect.objectContaining({
            'Content-Type': 'application/json',
          }),
        })
      );
    });

    it('updates profile state on success', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockProfile),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchProfile();
      });

      expect(result.current.profile).toEqual(mockProfile.user);
    });

    it('includes token in authorization header', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockProfile),
      });

      const { result } = renderHook(() =>
        useUserSettings({ autoFetch: false, token: 'test-token' })
      );

      await act(async () => {
        await result.current.fetchProfile();
      });

      expect(mockFetch).toHaveBeenCalledWith(
        '/auth/me',
        expect.objectContaining({
          headers: expect.objectContaining({
            Authorization: 'Bearer test-token',
          }),
        })
      );
    });

    it('uses custom baseUrl', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockProfile),
      });

      const { result } = renderHook(() =>
        useUserSettings({ autoFetch: false, baseUrl: 'https://api.example.com' })
      );

      await act(async () => {
        await result.current.fetchProfile();
      });

      expect(mockFetch).toHaveBeenCalledWith(
        'https://api.example.com/auth/me',
        expect.anything()
      );
    });

    it('sets error on fetch failure', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 401,
        text: () => Promise.resolve('Unauthorized'),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchProfile();
      });

      expect(result.current.error).toBe('Unauthorized');
    });

    it('sets loading state during fetch', async () => {
      let resolvePromise: (value: any) => void;
      const promise = new Promise((resolve) => {
        resolvePromise = resolve;
      });

      mockFetch.mockReturnValueOnce(promise);

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      act(() => {
        result.current.fetchProfile();
      });

      expect(result.current.loading).toBe(true);

      await act(async () => {
        resolvePromise!({
          ok: true,
          json: () => Promise.resolve(mockProfile),
        });
      });

      expect(result.current.loading).toBe(false);
    });
  });

  describe('updateProfile', () => {
    it('sends PUT request to profile endpoint', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockProfile),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.updateProfile({ name: 'New Name' });
      });

      expect(mockFetch).toHaveBeenCalledWith(
        '/auth/profile',
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ name: 'New Name' }),
        })
      );
    });

    it('returns true on success', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockProfile),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      let success: boolean = false;
      await act(async () => {
        success = await result.current.updateProfile({ name: 'New Name' });
      });

      expect(success).toBe(true);
    });

    it('returns false on failure', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        text: () => Promise.resolve('Invalid request'),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      let success: boolean = true;
      await act(async () => {
        success = await result.current.updateProfile({ name: 'New Name' });
      });

      expect(success).toBe(false);
    });

    it('updates profile state on success', async () => {
      const updatedProfile = {
        user: { ...mockProfile.user, name: 'Updated Name' },
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(updatedProfile),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.updateProfile({ name: 'Updated Name' });
      });

      expect(result.current.profile?.name).toBe('Updated Name');
    });
  });

  describe('changePassword', () => {
    it('sends PUT request to password endpoint', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ success: true }),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.changePassword({
          old_password: 'oldpass',
          new_password: 'newpass',
        });
      });

      expect(mockFetch).toHaveBeenCalledWith(
        '/auth/password',
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({
            old_password: 'oldpass',
            new_password: 'newpass',
          }),
        })
      );
    });

    it('returns true on success', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ success: true }),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      let success: boolean = false;
      await act(async () => {
        success = await result.current.changePassword({
          old_password: 'oldpass',
          new_password: 'newpass',
        });
      });

      expect(success).toBe(true);
    });

    it('returns false and sets error on failure', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 401,
        text: () => Promise.resolve('Wrong password'),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      let success: boolean = true;
      await act(async () => {
        success = await result.current.changePassword({
          old_password: 'wrongpass',
          new_password: 'newpass',
        });
      });

      expect(success).toBe(false);
      expect(result.current.error).toBe('Wrong password');
    });
  });

  describe('fetchLinkedAccounts', () => {
    it('fetches linked accounts from API', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockLinkedAccounts),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchLinkedAccounts();
      });

      expect(mockFetch).toHaveBeenCalledWith(
        '/auth/linked-accounts',
        expect.objectContaining({
          method: 'GET',
        })
      );
    });

    it('updates linkedAccounts state on success', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockLinkedAccounts),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchLinkedAccounts();
      });

      expect(result.current.linkedAccounts).toEqual(mockLinkedAccounts.accounts);
    });

    it('sets error on fetch failure', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.resolve('Server error'),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchLinkedAccounts();
      });

      expect(result.current.error).toBe('Server error');
    });
  });

  describe('unlinkAccount', () => {
    it('sends DELETE request to linked accounts endpoint', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ success: true }),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.unlinkAccount('github');
      });

      expect(mockFetch).toHaveBeenCalledWith(
        '/auth/linked-accounts/github',
        expect.objectContaining({
          method: 'DELETE',
        })
      );
    });

    it('removes account from local state on success', async () => {
      // First, populate linked accounts
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockLinkedAccounts),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchLinkedAccounts();
      });

      expect(result.current.linkedAccounts).toHaveLength(1);

      // Then, unlink account
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ success: true }),
      });

      await act(async () => {
        await result.current.unlinkAccount('github');
      });

      expect(result.current.linkedAccounts).toHaveLength(0);
    });

    it('returns true on success', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ success: true }),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      let success: boolean = false;
      await act(async () => {
        success = await result.current.unlinkAccount('github');
      });

      expect(success).toBe(true);
    });

    it('returns false on failure', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        text: () => Promise.resolve('Cannot unlink last account'),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      let success: boolean = true;
      await act(async () => {
        success = await result.current.unlinkAccount('github');
      });

      expect(success).toBe(false);
      expect(result.current.error).toBe('Cannot unlink last account');
    });
  });

  describe('error handling', () => {
    it('handles network errors gracefully', async () => {
      mockFetch.mockRejectedValueOnce(new Error('Network error'));

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchProfile();
      });

      expect(result.current.error).toBe('Network error');
    });

    it('clears previous error on new request', async () => {
      // First request fails
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.resolve('Server error'),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchProfile();
      });

      expect(result.current.error).toBe('Server error');

      // Second request succeeds
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockProfile),
      });

      await act(async () => {
        await result.current.fetchProfile();
      });

      expect(result.current.error).toBeNull();
    });

    it('handles empty error response', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.resolve(''),
      });

      const { result } = renderHook(() => useUserSettings({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchProfile();
      });

      expect(result.current.error).toBe('HTTP 500');
    });
  });
});
