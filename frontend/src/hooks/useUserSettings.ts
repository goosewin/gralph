import { useCallback, useState } from 'react';

/** User profile data */
export interface UserProfile {
  id: string;
  email: string;
  name?: string;
  role: 'admin' | 'developer' | 'viewer';
  created_at?: string;
}

/** Linked account from OAuth2 provider */
export interface LinkedAccount {
  provider: 'github' | 'google' | 'gitlab';
  provider_user_id: string;
  email: string;
  username?: string;
  linked_at: number;
}

/** Profile update request */
export interface UpdateProfileRequest {
  name?: string;
  email?: string;
}

/** Password change request */
export interface ChangePasswordRequest {
  old_password: string;
  new_password: string;
}

/** API response for profile endpoint */
export interface ProfileResponse {
  user: UserProfile;
}

/** API response for linked accounts endpoint */
export interface LinkedAccountsResponse {
  accounts: LinkedAccount[];
}

/** Hook options */
export interface UseUserSettingsOptions {
  baseUrl?: string;
  token?: string;
  autoFetch?: boolean;
}

/** Hook return type */
export interface UseUserSettingsReturn {
  profile: UserProfile | null;
  linkedAccounts: LinkedAccount[];
  loading: boolean;
  error: string | null;
  fetchProfile: () => Promise<void>;
  updateProfile: (data: UpdateProfileRequest) => Promise<boolean>;
  changePassword: (data: ChangePasswordRequest) => Promise<boolean>;
  fetchLinkedAccounts: () => Promise<void>;
  unlinkAccount: (provider: string) => Promise<boolean>;
}

/**
 * Hook for managing user settings: profile, password, and linked accounts.
 */
export function useUserSettings({
  baseUrl = '',
  token,
  autoFetch = true,
}: UseUserSettingsOptions = {}): UseUserSettingsReturn {
  const [profile, setProfile] = useState<UserProfile | null>(null);
  const [linkedAccounts, setLinkedAccounts] = useState<LinkedAccount[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
  };
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const fetchProfile = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/auth/me`, {
        method: 'GET',
        headers,
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || `HTTP ${response.status}`);
      }

      const data: ProfileResponse = await response.json();
      setProfile(data.user);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch profile');
    } finally {
      setLoading(false);
    }
  }, [baseUrl, token]);

  const updateProfile = useCallback(
    async (data: UpdateProfileRequest): Promise<boolean> => {
      setLoading(true);
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/auth/profile`, {
          method: 'PUT',
          headers,
          body: JSON.stringify(data),
        });

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `HTTP ${response.status}`);
        }

        const result: ProfileResponse = await response.json();
        setProfile(result.user);
        return true;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to update profile');
        return false;
      } finally {
        setLoading(false);
      }
    },
    [baseUrl, token]
  );

  const changePassword = useCallback(
    async (data: ChangePasswordRequest): Promise<boolean> => {
      setLoading(true);
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/auth/password`, {
          method: 'PUT',
          headers,
          body: JSON.stringify(data),
        });

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `HTTP ${response.status}`);
        }

        return true;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to change password');
        return false;
      } finally {
        setLoading(false);
      }
    },
    [baseUrl, token]
  );

  const fetchLinkedAccounts = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/auth/linked-accounts`, {
        method: 'GET',
        headers,
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || `HTTP ${response.status}`);
      }

      const data: LinkedAccountsResponse = await response.json();
      setLinkedAccounts(data.accounts);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch linked accounts');
    } finally {
      setLoading(false);
    }
  }, [baseUrl, token]);

  const unlinkAccount = useCallback(
    async (provider: string): Promise<boolean> => {
      setLoading(true);
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/auth/linked-accounts/${provider}`, {
          method: 'DELETE',
          headers,
        });

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `HTTP ${response.status}`);
        }

        // Remove from local state
        setLinkedAccounts((prev) => prev.filter((a) => a.provider !== provider));
        return true;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to unlink account');
        return false;
      } finally {
        setLoading(false);
      }
    },
    [baseUrl, token]
  );

  return {
    profile,
    linkedAccounts,
    loading,
    error,
    fetchProfile,
    updateProfile,
    changePassword,
    fetchLinkedAccounts,
    unlinkAccount,
  };
}
