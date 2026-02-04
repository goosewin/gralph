import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { UserSettings } from './UserSettings';

// Mock fetch globally
const mockFetch = vi.fn();
const originalFetch = globalThis.fetch;

describe('UserSettings', () => {
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
      {
        provider: 'google' as const,
        provider_user_id: '67890',
        email: 'test@gmail.com',
        username: undefined,
        linked_at: 1704153600,
      },
    ],
  };

  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.fetch = mockFetch as typeof fetch;
    mockFetch.mockImplementation((url: string) => {
      if (url.includes('/auth/me')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve(mockProfile),
        });
      }
      if (url.includes('/auth/linked-accounts') && !url.includes('/github') && !url.includes('/google') && !url.includes('/gitlab')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve(mockLinkedAccounts),
        });
      }
      if (url.includes('/auth/profile')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve(mockProfile),
        });
      }
      if (url.includes('/auth/password')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ success: true }),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve({}),
      });
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    globalThis.fetch = originalFetch;
  });

  describe('rendering', () => {
    it('renders the settings page title', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByText('User Settings')).toBeInTheDocument();
      });
    });

    it('renders profile section', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByText('Profile')).toBeInTheDocument();
        expect(screen.getByText('Update your personal information.')).toBeInTheDocument();
      });
    });

    it('renders password change section', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByText('Change Password')).toBeInTheDocument();
        expect(screen.getByText('Update your password. You will need to enter your current password.')).toBeInTheDocument();
      });
    });

    it('renders linked accounts section', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByText('Linked Accounts')).toBeInTheDocument();
        expect(screen.getByText('Manage your connected social accounts for quick sign-in.')).toBeInTheDocument();
      });
    });

    it('displays user profile data in form', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        const nameInput = screen.getByLabelText('Name') as HTMLInputElement;
        const emailInput = screen.getByLabelText('Email') as HTMLInputElement;

        expect(nameInput.value).toBe('Test User');
        expect(emailInput.value).toBe('test@example.com');
      });
    });

    it('displays user role badge', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByText('developer')).toBeInTheDocument();
      });
    });

    it('displays linked accounts list', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByText('GitHub')).toBeInTheDocument();
        expect(screen.getByText('testuser')).toBeInTheDocument();
        expect(screen.getByText('Google')).toBeInTheDocument();
        expect(screen.getByText('test@gmail.com')).toBeInTheDocument();
      });
    });
  });

  describe('profile form', () => {
    it('allows editing the name field', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Name')).toBeInTheDocument();
      });

      const nameInput = screen.getByLabelText('Name');
      await user.clear(nameInput);
      await user.type(nameInput, 'New Name');

      expect(nameInput).toHaveValue('New Name');
    });

    it('allows editing the email field', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Email')).toBeInTheDocument();
      });

      const emailInput = screen.getByLabelText('Email');
      await user.clear(emailInput);
      await user.type(emailInput, 'new@example.com');

      expect(emailInput).toHaveValue('new@example.com');
    });

    it('validates email format', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      // Wait for profile to be loaded into form
      await waitFor(() => {
        const emailInput = screen.getByLabelText('Email') as HTMLInputElement;
        expect(emailInput.value).toBe('test@example.com');
      });

      // Clear the mock to track only calls after this point
      mockFetch.mockClear();

      const emailInput = screen.getByLabelText('Email');

      // Clear and type using userEvent for proper event simulation
      await user.clear(emailInput);
      await user.type(emailInput, 'invalid-email');

      // Verify the value was properly set
      expect((emailInput as HTMLInputElement).value).toBe('invalid-email');

      const saveButton = screen.getByRole('button', { name: /save changes/i });
      await user.click(saveButton);

      // Wait a moment for any async operations
      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 100));
      });

      // Verify that the profile update API was NOT called (validation should prevent it)
      const profileUpdateCalls = mockFetch.mock.calls.filter(
        (call: [string, RequestInit?]) => call[0].includes('/auth/profile') && call[1]?.method === 'PUT'
      );
      expect(profileUpdateCalls).toHaveLength(0);
    });

    it('submits profile update successfully', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Name')).toBeInTheDocument();
      });

      const nameInput = screen.getByLabelText('Name');
      await user.clear(nameInput);
      await user.type(nameInput, 'Updated Name');

      const saveButton = screen.getByRole('button', { name: /save changes/i });
      await user.click(saveButton);

      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledWith(
          expect.stringContaining('/auth/profile'),
          expect.objectContaining({
            method: 'PUT',
            body: expect.stringContaining('Updated Name'),
          })
        );
      });
    });

    it('shows success message after profile update', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Name')).toBeInTheDocument();
      });

      const saveButton = screen.getByRole('button', { name: /save changes/i });
      await user.click(saveButton);

      await waitFor(() => {
        expect(screen.getByText('Profile updated successfully!')).toBeInTheDocument();
      });
    });
  });

  describe('password change form', () => {
    it('renders password fields', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
        expect(screen.getByLabelText('New Password')).toBeInTheDocument();
        expect(screen.getByLabelText('Confirm New Password')).toBeInTheDocument();
      });
    });

    it('validates required current password', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('New Password')).toBeInTheDocument();
      });

      const newPasswordInput = screen.getByLabelText('New Password');
      await user.type(newPasswordInput, 'newpassword123');

      const confirmPasswordInput = screen.getByLabelText('Confirm New Password');
      await user.type(confirmPasswordInput, 'newpassword123');

      const changeButton = screen.getByRole('button', { name: /change password/i });
      await user.click(changeButton);

      await waitFor(() => {
        expect(screen.getByText('Current password is required')).toBeInTheDocument();
      });
    });

    it('validates minimum password length', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
      });

      const oldPasswordInput = screen.getByLabelText('Current Password');
      await user.type(oldPasswordInput, 'oldpassword');

      const newPasswordInput = screen.getByLabelText('New Password');
      await user.type(newPasswordInput, 'short');

      const changeButton = screen.getByRole('button', { name: /change password/i });
      await user.click(changeButton);

      await waitFor(() => {
        expect(screen.getByText('Password must be at least 8 characters')).toBeInTheDocument();
      });
    });

    it('validates password confirmation matches', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
      });

      const oldPasswordInput = screen.getByLabelText('Current Password');
      await user.type(oldPasswordInput, 'oldpassword');

      const newPasswordInput = screen.getByLabelText('New Password');
      await user.type(newPasswordInput, 'newpassword123');

      const confirmPasswordInput = screen.getByLabelText('Confirm New Password');
      await user.type(confirmPasswordInput, 'differentpassword');

      const changeButton = screen.getByRole('button', { name: /change password/i });
      await user.click(changeButton);

      await waitFor(() => {
        expect(screen.getByText('Passwords do not match')).toBeInTheDocument();
      });
    });

    it('submits password change successfully', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
      });

      const oldPasswordInput = screen.getByLabelText('Current Password');
      await user.type(oldPasswordInput, 'oldpassword');

      const newPasswordInput = screen.getByLabelText('New Password');
      await user.type(newPasswordInput, 'newpassword123');

      const confirmPasswordInput = screen.getByLabelText('Confirm New Password');
      await user.type(confirmPasswordInput, 'newpassword123');

      const changeButton = screen.getByRole('button', { name: /change password/i });
      await user.click(changeButton);

      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledWith(
          expect.stringContaining('/auth/password'),
          expect.objectContaining({
            method: 'PUT',
            body: expect.stringContaining('oldpassword'),
          })
        );
      });
    });

    it('clears password fields after successful change', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
      });

      const oldPasswordInput = screen.getByLabelText('Current Password') as HTMLInputElement;
      await user.type(oldPasswordInput, 'oldpassword');

      const newPasswordInput = screen.getByLabelText('New Password') as HTMLInputElement;
      await user.type(newPasswordInput, 'newpassword123');

      const confirmPasswordInput = screen.getByLabelText('Confirm New Password') as HTMLInputElement;
      await user.type(confirmPasswordInput, 'newpassword123');

      const changeButton = screen.getByRole('button', { name: /change password/i });
      await user.click(changeButton);

      await waitFor(() => {
        expect(oldPasswordInput.value).toBe('');
        expect(newPasswordInput.value).toBe('');
        expect(confirmPasswordInput.value).toBe('');
      });
    });

    it('toggles password visibility', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
      });

      const currentPasswordInput = screen.getByLabelText('Current Password');
      expect(currentPasswordInput).toHaveAttribute('type', 'password');

      const showPasswordCheckbox = screen.getByRole('checkbox', { name: /show passwords/i });
      await user.click(showPasswordCheckbox);

      expect(currentPasswordInput).toHaveAttribute('type', 'text');

      await user.click(showPasswordCheckbox);
      expect(currentPasswordInput).toHaveAttribute('type', 'password');
    });

    it('shows success message after password change', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
      });

      const oldPasswordInput = screen.getByLabelText('Current Password');
      await user.type(oldPasswordInput, 'oldpassword');

      const newPasswordInput = screen.getByLabelText('New Password');
      await user.type(newPasswordInput, 'newpassword123');

      const confirmPasswordInput = screen.getByLabelText('Confirm New Password');
      await user.type(confirmPasswordInput, 'newpassword123');

      const changeButton = screen.getByRole('button', { name: /change password/i });
      await user.click(changeButton);

      await waitFor(() => {
        expect(screen.getByText('Password changed successfully!')).toBeInTheDocument();
      });
    });
  });

  describe('linked accounts', () => {
    it('displays provider icons', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByText('🐙')).toBeInTheDocument();
        expect(screen.getByText('🔍')).toBeInTheDocument();
      });
    });

    it('displays linked account dates', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        const dateElements = screen.getAllByText(/Linked/);
        expect(dateElements.length).toBeGreaterThan(0);
      });
    });

    it('shows unlink confirmation on first click', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Unlink GitHub')).toBeInTheDocument();
      });

      const unlinkButton = screen.getByLabelText('Unlink GitHub');
      await user.click(unlinkButton);

      await waitFor(() => {
        expect(screen.getByText('Unlink this account?')).toBeInTheDocument();
        expect(screen.getByRole('button', { name: /confirm/i })).toBeInTheDocument();
        expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
      });
    });

    it('cancels unlink on cancel button click', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Unlink GitHub')).toBeInTheDocument();
      });

      const unlinkButton = screen.getByLabelText('Unlink GitHub');
      await user.click(unlinkButton);

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
      });

      const cancelButton = screen.getByRole('button', { name: /cancel/i });
      await user.click(cancelButton);

      await waitFor(() => {
        expect(screen.queryByText('Unlink this account?')).not.toBeInTheDocument();
      });
    });

    it('unlinks account on confirm', async () => {
      mockFetch.mockImplementation((url: string, options?: RequestInit) => {
        if (url.includes('/auth/me')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockProfile),
          });
        }
        if (url.includes('/auth/linked-accounts/github') && options?.method === 'DELETE') {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ success: true }),
          });
        }
        if (url.includes('/auth/linked-accounts')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockLinkedAccounts),
          });
        }
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({}),
        });
      });

      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Unlink GitHub')).toBeInTheDocument();
      });

      const unlinkButton = screen.getByLabelText('Unlink GitHub');
      await user.click(unlinkButton);

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /confirm unlink github/i })).toBeInTheDocument();
      });

      const confirmButton = screen.getByRole('button', { name: /confirm unlink github/i });
      await user.click(confirmButton);

      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledWith(
          expect.stringContaining('/auth/linked-accounts/github'),
          expect.objectContaining({
            method: 'DELETE',
          })
        );
      });
    });

    it('shows empty state when no linked accounts', async () => {
      mockFetch.mockImplementation((url: string) => {
        if (url.includes('/auth/me')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockProfile),
          });
        }
        if (url.includes('/auth/linked-accounts')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ accounts: [] }),
          });
        }
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({}),
        });
      });

      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByText('No linked accounts.')).toBeInTheDocument();
        expect(screen.getByText('Link your social accounts for faster sign-in.')).toBeInTheDocument();
      });
    });
  });

  describe('error handling', () => {
    it('displays error when profile fetch fails', async () => {
      mockFetch.mockImplementation((url: string) => {
        if (url.includes('/auth/me')) {
          return Promise.resolve({
            ok: false,
            status: 401,
            text: () => Promise.resolve('Unauthorized'),
          });
        }
        if (url.includes('/auth/linked-accounts')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ accounts: [] }),
          });
        }
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({}),
        });
      });

      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByRole('alert')).toBeInTheDocument();
        expect(screen.getByText('Unauthorized')).toBeInTheDocument();
      });
    });

    it('displays error when profile update fails', async () => {
      mockFetch.mockImplementation((url: string, options?: RequestInit) => {
        if (url.includes('/auth/me')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockProfile),
          });
        }
        if (url.includes('/auth/linked-accounts')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockLinkedAccounts),
          });
        }
        if (url.includes('/auth/profile') && options?.method === 'PUT') {
          return Promise.resolve({
            ok: false,
            status: 400,
            text: () => Promise.resolve('Invalid email format'),
          });
        }
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({}),
        });
      });

      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Name')).toBeInTheDocument();
      });

      const saveButton = screen.getByRole('button', { name: /save changes/i });
      await user.click(saveButton);

      await waitFor(() => {
        expect(screen.getByRole('alert')).toBeInTheDocument();
        expect(screen.getByText('Invalid email format')).toBeInTheDocument();
      });
    });

    it('displays error when password change fails', async () => {
      mockFetch.mockImplementation((url: string, options?: RequestInit) => {
        if (url.includes('/auth/me')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockProfile),
          });
        }
        if (url.includes('/auth/linked-accounts')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockLinkedAccounts),
          });
        }
        if (url.includes('/auth/password') && options?.method === 'PUT') {
          return Promise.resolve({
            ok: false,
            status: 401,
            text: () => Promise.resolve('Current password is incorrect'),
          });
        }
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({}),
        });
      });

      const user = userEvent.setup();
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
      });

      const oldPasswordInput = screen.getByLabelText('Current Password');
      await user.type(oldPasswordInput, 'wrongpassword');

      const newPasswordInput = screen.getByLabelText('New Password');
      await user.type(newPasswordInput, 'newpassword123');

      const confirmPasswordInput = screen.getByLabelText('Confirm New Password');
      await user.type(confirmPasswordInput, 'newpassword123');

      const changeButton = screen.getByRole('button', { name: /change password/i });
      await user.click(changeButton);

      await waitFor(() => {
        expect(screen.getByRole('alert')).toBeInTheDocument();
        expect(screen.getByText('Current password is incorrect')).toBeInTheDocument();
      });
    });
  });

  describe('accessibility', () => {
    it('has proper form labels', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Name')).toBeInTheDocument();
        expect(screen.getByLabelText('Email')).toBeInTheDocument();
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
        expect(screen.getByLabelText('New Password')).toBeInTheDocument();
        expect(screen.getByLabelText('Confirm New Password')).toBeInTheDocument();
      });
    });

    it('has proper section headings', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'User Settings' })).toBeInTheDocument();
        expect(screen.getByRole('heading', { name: 'Profile' })).toBeInTheDocument();
        expect(screen.getByRole('heading', { name: 'Change Password' })).toBeInTheDocument();
        expect(screen.getByRole('heading', { name: 'Linked Accounts' })).toBeInTheDocument();
      });
    });

    it('has aria-describedby for error messages', async () => {
      const user = userEvent.setup();
      render(<UserSettings />);

      // Wait for profile to be loaded into form
      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
      });

      // Test with password validation which reliably shows error messages
      const newPasswordInput = screen.getByLabelText('New Password');
      await user.type(newPasswordInput, 'short');

      const confirmPasswordInput = screen.getByLabelText('Confirm New Password');
      await user.type(confirmPasswordInput, 'short');

      const changeButton = screen.getByRole('button', { name: /change password/i });
      await user.click(changeButton);

      // Check for aria-describedby on the old password field which should have "Current password is required" error
      const oldPasswordInput = screen.getByLabelText('Current Password');
      await waitFor(() => {
        expect(oldPasswordInput).toHaveAttribute('aria-describedby', 'old-password-error');
      });
    });

    it('has proper button labels for screen readers', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Unlink GitHub')).toBeInTheDocument();
        expect(screen.getByLabelText('Unlink Google')).toBeInTheDocument();
      });
    });

    it('linked accounts list has proper role', async () => {
      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByRole('list')).toBeInTheDocument();
      });
    });
  });

  describe('loading states', () => {
    it('disables save button while loading', async () => {
      const user = userEvent.setup();

      // Make profile update take some time
      mockFetch.mockImplementation((url: string, options?: RequestInit) => {
        if (url.includes('/auth/me')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockProfile),
          });
        }
        if (url.includes('/auth/linked-accounts')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockLinkedAccounts),
          });
        }
        if (url.includes('/auth/profile') && options?.method === 'PUT') {
          return new Promise((resolve) => {
            setTimeout(() => {
              resolve({
                ok: true,
                json: () => Promise.resolve(mockProfile),
              });
            }, 100);
          });
        }
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({}),
        });
      });

      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Name')).toBeInTheDocument();
      });

      const saveButton = screen.getByRole('button', { name: /save changes/i });
      await user.click(saveButton);

      // Button should show loading state
      expect(screen.getByRole('button', { name: /saving/i })).toBeInTheDocument();
    });

    it('disables change password button while loading', async () => {
      const user = userEvent.setup();

      mockFetch.mockImplementation((url: string, options?: RequestInit) => {
        if (url.includes('/auth/me')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockProfile),
          });
        }
        if (url.includes('/auth/linked-accounts')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockLinkedAccounts),
          });
        }
        if (url.includes('/auth/password') && options?.method === 'PUT') {
          return new Promise((resolve) => {
            setTimeout(() => {
              resolve({
                ok: true,
                json: () => Promise.resolve({ success: true }),
              });
            }, 100);
          });
        }
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({}),
        });
      });

      render(<UserSettings />);

      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
      });

      const oldPasswordInput = screen.getByLabelText('Current Password');
      await user.type(oldPasswordInput, 'oldpassword');

      const newPasswordInput = screen.getByLabelText('New Password');
      await user.type(newPasswordInput, 'newpassword123');

      const confirmPasswordInput = screen.getByLabelText('Confirm New Password');
      await user.type(confirmPasswordInput, 'newpassword123');

      const changeButton = screen.getByRole('button', { name: /change password/i });
      await user.click(changeButton);

      // Button should show loading state
      expect(screen.getByRole('button', { name: /changing/i })).toBeInTheDocument();
    });
  });
});
