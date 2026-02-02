import { useCallback, useEffect, useState } from 'react';

export type UserRole = 'admin' | 'developer' | 'viewer';

export interface AdminUser {
  id: string;
  email: string;
  role: UserRole;
  created_at: number;
  updated_at: number;
}

export interface AdminUsersListResponse {
  users: AdminUser[];
  total: number;
  offset: number;
  limit: number;
}

export interface BulkRoleResult {
  success_count: number;
  failure_count: number;
  failures: Array<{ user_id: string; error: string }>;
}

export interface UseAdminUsersOptions {
  baseUrl?: string;
  token?: string;
  autoFetch?: boolean;
}

export interface UseAdminUsersResult {
  users: AdminUser[];
  total: number;
  loading: boolean;
  error: string | null;
  offset: number;
  limit: number;
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  setPage: (page: number) => void;
  fetchUsers: () => Promise<void>;
  updateUserRole: (userId: string, newRole: UserRole) => Promise<boolean>;
  deleteUser: (userId: string) => Promise<boolean>;
  bulkUpdateRoles: (userIds: string[], newRole: UserRole) => Promise<BulkRoleResult | null>;
}

export function useAdminUsers({
  baseUrl = '',
  token,
  autoFetch = true,
}: UseAdminUsersOptions = {}): UseAdminUsersResult {
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const [limit] = useState(50);
  const [searchQuery, setSearchQuery] = useState('');

  const getHeaders = useCallback(() => {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }
    return headers;
  }, [token]);

  const fetchUsers = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const params = new URLSearchParams({
        offset: offset.toString(),
        limit: limit.toString(),
      });
      if (searchQuery) {
        params.set('search', searchQuery);
      }

      const response = await fetch(`${baseUrl}/admin/users?${params.toString()}`, {
        headers: getHeaders(),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      const data: AdminUsersListResponse = await response.json();
      setUsers(data.users);
      setTotal(data.total);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch users');
    } finally {
      setLoading(false);
    }
  }, [baseUrl, getHeaders, offset, limit, searchQuery]);

  const updateUserRole = useCallback(
    async (userId: string, newRole: UserRole): Promise<boolean> => {
      setError(null);

      try {
        const response = await fetch(`${baseUrl}/admin/users/${encodeURIComponent(userId)}/role`, {
          method: 'PUT',
          headers: getHeaders(),
          body: JSON.stringify({ role: newRole }),
        });

        if (!response.ok) {
          const errorData = await response.json().catch(() => ({}));
          throw new Error(errorData.error || `HTTP ${response.status}`);
        }

        const updatedUser: AdminUser = await response.json();

        // Update local state
        setUsers((prev) =>
          prev.map((u) => (u.id === userId ? updatedUser : u))
        );

        return true;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to update user role');
        return false;
      }
    },
    [baseUrl, getHeaders]
  );

  const deleteUser = useCallback(
    async (userId: string): Promise<boolean> => {
      setError(null);

      try {
        const response = await fetch(`${baseUrl}/admin/users/${encodeURIComponent(userId)}`, {
          method: 'DELETE',
          headers: getHeaders(),
        });

        if (!response.ok) {
          const errorData = await response.json().catch(() => ({}));
          throw new Error(errorData.error || `HTTP ${response.status}`);
        }

        // Update local state
        setUsers((prev) => prev.filter((u) => u.id !== userId));
        setTotal((prev) => prev - 1);

        return true;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to delete user');
        return false;
      }
    },
    [baseUrl, getHeaders]
  );

  const bulkUpdateRoles = useCallback(
    async (userIds: string[], newRole: UserRole): Promise<BulkRoleResult | null> => {
      setError(null);

      try {
        const response = await fetch(`${baseUrl}/admin/users/bulk-role`, {
          method: 'POST',
          headers: getHeaders(),
          body: JSON.stringify({ user_ids: userIds, role: newRole }),
        });

        if (!response.ok) {
          const errorData = await response.json().catch(() => ({}));
          throw new Error(errorData.error || `HTTP ${response.status}`);
        }

        const result: BulkRoleResult = await response.json();

        // Refresh the user list to get updated roles
        await fetchUsers();

        return result;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to update roles');
        return null;
      }
    },
    [baseUrl, getHeaders, fetchUsers]
  );

  const setPage = useCallback((page: number) => {
    setOffset(page * limit);
  }, [limit]);

  useEffect(() => {
    if (autoFetch) {
      fetchUsers();
    }
  }, [autoFetch, fetchUsers]);

  return {
    users,
    total,
    loading,
    error,
    offset,
    limit,
    searchQuery,
    setSearchQuery,
    setPage,
    fetchUsers,
    updateUserRole,
    deleteUser,
    bulkUpdateRoles,
  };
}
