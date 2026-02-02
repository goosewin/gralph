import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useAdminUsers } from './useAdminUsers';

// Mock fetch globally
const mockFetch = vi.fn();
(globalThis as unknown as { fetch: typeof fetch }).fetch = mockFetch;

describe('useAdminUsers', () => {
  const mockUsersResponse = {
    users: [
      {
        id: 'user-1',
        email: 'admin@example.com',
        role: 'admin' as const,
        created_at: 1704067200,
        updated_at: 1704067200,
      },
      {
        id: 'user-2',
        email: 'developer@example.com',
        role: 'developer' as const,
        created_at: 1704153600,
        updated_at: 1704153600,
      },
    ],
    total: 2,
    offset: 0,
    limit: 50,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockUsersResponse),
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('initial state', () => {
    it('starts with empty users array', () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));
      expect(result.current.users).toEqual([]);
    });

    it('starts with total of 0', () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));
      expect(result.current.total).toBe(0);
    });

    it('starts with loading false', () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));
      expect(result.current.loading).toBe(false);
    });

    it('starts with null error', () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));
      expect(result.current.error).toBeNull();
    });

    it('starts with offset 0', () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));
      expect(result.current.offset).toBe(0);
    });

    it('starts with empty search query', () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));
      expect(result.current.searchQuery).toBe('');
    });
  });

  describe('fetchUsers', () => {
    it('fetches users from API', async () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      expect(mockFetch).toHaveBeenCalledWith(
        '/admin/users?offset=0&limit=50',
        expect.objectContaining({
          headers: expect.objectContaining({
            'Content-Type': 'application/json',
          }),
        })
      );
    });

    it('updates users state on success', async () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      expect(result.current.users).toEqual(mockUsersResponse.users);
      expect(result.current.total).toBe(2);
    });

    it('sets loading true during fetch', async () => {
      let resolvePromise: (value: unknown) => void;
      const promise = new Promise((resolve) => {
        resolvePromise = resolve;
      });

      mockFetch.mockReturnValue(promise);

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      act(() => {
        result.current.fetchUsers();
      });

      expect(result.current.loading).toBe(true);

      await act(async () => {
        resolvePromise!({
          ok: true,
          json: () => Promise.resolve(mockUsersResponse),
        });
      });

      expect(result.current.loading).toBe(false);
    });

    it('sets error on API failure', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        json: () => Promise.resolve({ error: 'Permission denied' }),
      });

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      expect(result.current.error).toBe('Permission denied');
    });

    it('includes search query in request', async () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      act(() => {
        result.current.setSearchQuery('admin');
      });

      await act(async () => {
        await result.current.fetchUsers();
      });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('search=admin'),
        expect.any(Object)
      );
    });

    it('includes token in Authorization header when provided', async () => {
      const { result } = renderHook(() =>
        useAdminUsers({ autoFetch: false, token: 'test-token' })
      );

      await act(async () => {
        await result.current.fetchUsers();
      });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({
            Authorization: 'Bearer test-token',
          }),
        })
      );
    });
  });

  describe('updateUserRole', () => {
    it('sends PUT request to update role', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            ...mockUsersResponse.users[1],
            role: 'admin',
          }),
      });

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      await act(async () => {
        const success = await result.current.updateUserRole('user-2', 'admin');
        expect(success).toBe(true);
      });

      expect(mockFetch).toHaveBeenCalledWith(
        '/admin/users/user-2/role',
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ role: 'admin' }),
        })
      );
    });

    it('updates local state on success', async () => {
      const updatedUser = {
        ...mockUsersResponse.users[1],
        role: 'admin' as const,
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(updatedUser),
      });

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      await act(async () => {
        await result.current.updateUserRole('user-2', 'admin');
      });

      const updatedUserInState = result.current.users.find(
        (u) => u.id === 'user-2'
      );
      expect(updatedUserInState?.role).toBe('admin');
    });

    it('returns false on failure', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });
      mockFetch.mockResolvedValueOnce({
        ok: false,
        json: () => Promise.resolve({ error: 'User not found' }),
      });

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      let success: boolean;
      await act(async () => {
        success = await result.current.updateUserRole('user-99', 'admin');
      });

      expect(success!).toBe(false);
      expect(result.current.error).toBe('User not found');
    });
  });

  describe('deleteUser', () => {
    it('sends DELETE request', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            message: 'User deleted successfully',
            deleted_user: { id: 'user-2', email: 'developer@example.com' },
          }),
      });

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      await act(async () => {
        const success = await result.current.deleteUser('user-2');
        expect(success).toBe(true);
      });

      expect(mockFetch).toHaveBeenCalledWith(
        '/admin/users/user-2',
        expect.objectContaining({
          method: 'DELETE',
        })
      );
    });

    it('removes user from local state on success', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            message: 'User deleted successfully',
            deleted_user: { id: 'user-2', email: 'developer@example.com' },
          }),
      });

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      expect(result.current.users).toHaveLength(2);

      await act(async () => {
        await result.current.deleteUser('user-2');
      });

      expect(result.current.users).toHaveLength(1);
      expect(result.current.users.find((u) => u.id === 'user-2')).toBeUndefined();
    });

    it('decrements total on success', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            message: 'User deleted successfully',
            deleted_user: { id: 'user-2', email: 'developer@example.com' },
          }),
      });

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      expect(result.current.total).toBe(2);

      await act(async () => {
        await result.current.deleteUser('user-2');
      });

      expect(result.current.total).toBe(1);
    });
  });

  describe('bulkUpdateRoles', () => {
    it('sends POST request with user IDs and role', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            success_count: 2,
            failure_count: 0,
            failures: [],
          }),
      });
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      await act(async () => {
        const response = await result.current.bulkUpdateRoles(
          ['user-1', 'user-2'],
          'viewer'
        );
        expect(response).toEqual({
          success_count: 2,
          failure_count: 0,
          failures: [],
        });
      });

      expect(mockFetch).toHaveBeenCalledWith(
        '/admin/users/bulk-role',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ user_ids: ['user-1', 'user-2'], role: 'viewer' }),
        })
      );
    });

    it('refetches users after bulk update', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            success_count: 1,
            failure_count: 0,
            failures: [],
          }),
      });
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      const initialCallCount = mockFetch.mock.calls.length;

      await act(async () => {
        await result.current.bulkUpdateRoles(['user-1'], 'viewer');
      });

      // Should have made 2 more calls: bulk update + refetch
      expect(mockFetch.mock.calls.length).toBe(initialCallCount + 2);
    });

    it('returns null on failure', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockUsersResponse),
      });
      mockFetch.mockResolvedValueOnce({
        ok: false,
        json: () => Promise.resolve({ error: 'Permission denied' }),
      });

      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      await act(async () => {
        await result.current.fetchUsers();
      });

      let response: unknown;
      await act(async () => {
        response = await result.current.bulkUpdateRoles(['user-1'], 'viewer');
      });

      expect(response).toBeNull();
      expect(result.current.error).toBe('Permission denied');
    });
  });

  describe('pagination', () => {
    it('setPage updates offset', async () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      act(() => {
        result.current.setPage(2);
      });

      expect(result.current.offset).toBe(100); // page 2 * limit 50
    });

    it('includes offset in fetch request', async () => {
      const { result } = renderHook(() => useAdminUsers({ autoFetch: false }));

      act(() => {
        result.current.setPage(1);
      });

      await act(async () => {
        await result.current.fetchUsers();
      });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('offset=50'),
        expect.any(Object)
      );
    });
  });

  describe('autoFetch', () => {
    it('fetches users automatically when autoFetch is true', async () => {
      renderHook(() => useAdminUsers({ autoFetch: true }));

      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledWith(
          expect.stringContaining('/admin/users'),
          expect.any(Object)
        );
      });
    });

    it('does not fetch automatically when autoFetch is false', () => {
      renderHook(() => useAdminUsers({ autoFetch: false }));

      expect(mockFetch).not.toHaveBeenCalled();
    });
  });
});
