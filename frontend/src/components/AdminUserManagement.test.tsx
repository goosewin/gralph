import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { AdminUserManagement } from './AdminUserManagement';

// Mock fetch globally
const mockFetch = vi.fn();
(globalThis as unknown as { fetch: typeof fetch }).fetch = mockFetch;

describe('AdminUserManagement', () => {
  const mockUsers = {
    users: [
      {
        id: 'user-1',
        email: 'admin@example.com',
        role: 'admin',
        created_at: 1704067200,
        updated_at: 1704067200,
      },
      {
        id: 'user-2',
        email: 'developer@example.com',
        role: 'developer',
        created_at: 1704153600,
        updated_at: 1704153600,
      },
      {
        id: 'user-3',
        email: 'viewer@example.com',
        role: 'viewer',
        created_at: 1704240000,
        updated_at: 1704240000,
      },
    ],
    total: 3,
    offset: 0,
    limit: 50,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockFetch.mockImplementation((url: string) => {
      if (url.includes('/admin/users') && !url.includes('/role') && !url.includes('/bulk-role')) {
        const urlObj = new URL(url, 'http://localhost');
        const search = urlObj.searchParams.get('search');
        if (search) {
          const filtered = mockUsers.users.filter((u) =>
            u.email.toLowerCase().includes(search.toLowerCase())
          );
          return Promise.resolve({
            ok: true,
            json: () =>
              Promise.resolve({
                users: filtered,
                total: filtered.length,
                offset: 0,
                limit: 50,
              }),
          });
        }
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve(mockUsers),
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
  });

  describe('rendering', () => {
    it('renders the page title', async () => {
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByText('User Management')).toBeInTheDocument();
      });
    });

    it('renders search input', async () => {
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByLabelText('Search users')).toBeInTheDocument();
      });
    });

    it('renders user table headers', async () => {
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByText('Email')).toBeInTheDocument();
        expect(screen.getByText('Role')).toBeInTheDocument();
        expect(screen.getByText('Created')).toBeInTheDocument();
        expect(screen.getByText('Actions')).toBeInTheDocument();
      });
    });

    it('displays user list', async () => {
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByText('admin@example.com')).toBeInTheDocument();
        expect(screen.getByText('developer@example.com')).toBeInTheDocument();
        expect(screen.getByText('viewer@example.com')).toBeInTheDocument();
      });
    });

    it('displays user count', async () => {
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByText('3 users found')).toBeInTheDocument();
      });
    });

    it('marks current user with (you) badge', async () => {
      render(<AdminUserManagement currentUserId="user-1" />);

      await waitFor(() => {
        expect(screen.getByText('(you)')).toBeInTheDocument();
      });
    });
  });

  describe('search functionality', () => {
    it('filters users by search query', async () => {
      const user = userEvent.setup();
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByText('admin@example.com')).toBeInTheDocument();
      });

      const searchInput = screen.getByLabelText('Search users');
      await user.type(searchInput, 'admin');

      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledWith(
          expect.stringContaining('search=admin'),
          expect.any(Object)
        );
      });
    });

    it('shows search results count', async () => {
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByText('3 users found')).toBeInTheDocument();
      });
    });
  });

  describe('role management', () => {
    it('renders role dropdown for each user', async () => {
      render(<AdminUserManagement />);

      await waitFor(() => {
        const roleSelects = screen.getAllByRole('combobox');
        expect(roleSelects.length).toBeGreaterThan(0);
      });
    });

    it('disables role dropdown for current user', async () => {
      render(<AdminUserManagement currentUserId="user-1" />);

      await waitFor(() => {
        const roleSelects = screen.getAllByRole('combobox');
        const adminSelect = roleSelects[0];
        expect(adminSelect).toBeDisabled();
      });
    });

    it('updates user role on selection change', async () => {
      mockFetch.mockImplementation((url: string) => {
        if (url.includes('/admin/users') && url.includes('/role')) {
          return Promise.resolve({
            ok: true,
            json: () =>
              Promise.resolve({
                id: 'user-2',
                email: 'developer@example.com',
                role: 'admin',
                created_at: 1704153600,
                updated_at: 1704200000,
              }),
          });
        }
        if (url.includes('/admin/users')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockUsers),
          });
        }
        return Promise.resolve({ ok: true, json: () => Promise.resolve({}) });
      });

      const user = userEvent.setup();
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByText('developer@example.com')).toBeInTheDocument();
      });

      // Find the role select for developer user
      const roleSelects = screen.getAllByRole('combobox');
      const developerSelect = roleSelects[1]; // Second user

      await user.selectOptions(developerSelect, 'admin');

      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledWith(
          expect.stringContaining('/admin/users/user-2/role'),
          expect.objectContaining({
            method: 'PUT',
            body: JSON.stringify({ role: 'admin' }),
          })
        );
      });
    });
  });

  describe('user selection', () => {
    it('allows selecting multiple users', async () => {
      const user = userEvent.setup();
      render(<AdminUserManagement currentUserId="user-1" />);

      await waitFor(() => {
        expect(screen.getByText('developer@example.com')).toBeInTheDocument();
      });

      // Find checkboxes for non-current users
      const checkboxes = screen.getAllByRole('checkbox');
      // First checkbox is "select all", then one for each user
      // Current user checkbox should be disabled
      await user.click(checkboxes[2]); // Select developer

      expect(screen.getByText('1 selected')).toBeInTheDocument();
    });

    it('shows bulk actions when users are selected', async () => {
      const user = userEvent.setup();
      render(<AdminUserManagement currentUserId="user-1" />);

      await waitFor(() => {
        expect(screen.getByText('developer@example.com')).toBeInTheDocument();
      });

      const checkboxes = screen.getAllByRole('checkbox');
      await user.click(checkboxes[2]); // Select developer

      expect(screen.getByText('Apply Role')).toBeInTheDocument();
    });

    it('allows select all (excluding current user)', async () => {
      const user = userEvent.setup();
      render(<AdminUserManagement currentUserId="user-1" />);

      await waitFor(() => {
        expect(screen.getByText('developer@example.com')).toBeInTheDocument();
      });

      const selectAllCheckbox = screen.getByRole('checkbox', {
        name: /select all/i,
      });
      await user.click(selectAllCheckbox);

      expect(screen.getByText('2 selected')).toBeInTheDocument();
    });
  });

  describe('bulk role update', () => {
    it('calls bulk update API with selected users', async () => {
      mockFetch.mockImplementation((url: string) => {
        if (url.includes('/admin/users/bulk-role')) {
          return Promise.resolve({
            ok: true,
            json: () =>
              Promise.resolve({
                success_count: 2,
                failure_count: 0,
                failures: [],
              }),
          });
        }
        if (url.includes('/admin/users')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockUsers),
          });
        }
        return Promise.resolve({ ok: true, json: () => Promise.resolve({}) });
      });

      const user = userEvent.setup();
      render(<AdminUserManagement currentUserId="user-1" />);

      await waitFor(() => {
        expect(screen.getByText('developer@example.com')).toBeInTheDocument();
      });

      // Select users
      const checkboxes = screen.getAllByRole('checkbox');
      await user.click(checkboxes[2]); // Select developer
      await user.click(checkboxes[3]); // Select viewer

      // Select bulk role
      const bulkRoleSelect = screen.getByLabelText('Bulk role assignment');
      await user.selectOptions(bulkRoleSelect, 'admin');

      // Apply
      const applyButton = screen.getByText('Apply Role');
      await user.click(applyButton);

      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledWith(
          expect.stringContaining('/admin/users/bulk-role'),
          expect.objectContaining({
            method: 'POST',
            body: expect.stringContaining('user-2'),
          })
        );
      });
    });
  });

  describe('user deletion', () => {
    it('shows delete confirmation on first click', async () => {
      const user = userEvent.setup();
      render(<AdminUserManagement currentUserId="user-1" />);

      await waitFor(() => {
        expect(screen.getByText('developer@example.com')).toBeInTheDocument();
      });

      const deleteButtons = screen.getAllByText('Delete');
      await user.click(deleteButtons[0]); // Click first delete button

      expect(screen.getByText('Delete?')).toBeInTheDocument();
      expect(screen.getByText('Yes')).toBeInTheDocument();
      expect(screen.getByText('No')).toBeInTheDocument();
    });

    it('cancels delete on No click', async () => {
      const user = userEvent.setup();
      render(<AdminUserManagement currentUserId="user-1" />);

      await waitFor(() => {
        expect(screen.getByText('developer@example.com')).toBeInTheDocument();
      });

      const deleteButtons = screen.getAllByText('Delete');
      await user.click(deleteButtons[0]);

      const noButton = screen.getByText('No');
      await user.click(noButton);

      expect(screen.queryByText('Delete?')).not.toBeInTheDocument();
    });

    it('deletes user on confirmation', async () => {
      mockFetch.mockImplementation((url: string, options?: RequestInit) => {
        if (
          options?.method === 'DELETE' &&
          url.includes('/admin/users/')
        ) {
          return Promise.resolve({
            ok: true,
            json: () =>
              Promise.resolve({
                message: 'User deleted successfully',
                deleted_user: {
                  id: 'user-2',
                  email: 'developer@example.com',
                },
              }),
          });
        }
        if (url.includes('/admin/users')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockUsers),
          });
        }
        return Promise.resolve({ ok: true, json: () => Promise.resolve({}) });
      });

      const user = userEvent.setup();
      render(<AdminUserManagement currentUserId="user-1" />);

      await waitFor(() => {
        expect(screen.getByText('developer@example.com')).toBeInTheDocument();
      });

      const deleteButtons = screen.getAllByText('Delete');
      await user.click(deleteButtons[0]); // Show confirmation

      const yesButton = screen.getByText('Yes');
      await user.click(yesButton); // Confirm

      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledWith(
          expect.stringContaining('/admin/users/user-2'),
          expect.objectContaining({
            method: 'DELETE',
          })
        );
      });
    });

    it('does not show delete button for current user', async () => {
      render(<AdminUserManagement currentUserId="user-1" />);

      await waitFor(() => {
        expect(screen.getByText('admin@example.com')).toBeInTheDocument();
      });

      // The current user row should show "-" instead of delete button
      const rows = screen.getAllByRole('row');
      // First row is header, second is admin (current user)
      const adminRow = rows[1];
      expect(adminRow).toHaveTextContent('-');
    });
  });

  describe('pagination', () => {
    it('renders pagination when total exceeds limit', async () => {
      mockFetch.mockImplementation((url: string) => {
        if (url.includes('/admin/users')) {
          return Promise.resolve({
            ok: true,
            json: () =>
              Promise.resolve({
                users: mockUsers.users,
                total: 100,
                offset: 0,
                limit: 50,
              }),
          });
        }
        return Promise.resolve({ ok: true, json: () => Promise.resolve({}) });
      });

      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByText('Page 1 of 2')).toBeInTheDocument();
        expect(screen.getByText('Previous')).toBeInTheDocument();
        expect(screen.getByText('Next')).toBeInTheDocument();
      });
    });

    it('disables previous button on first page', async () => {
      mockFetch.mockImplementation((url: string) => {
        if (url.includes('/admin/users')) {
          return Promise.resolve({
            ok: true,
            json: () =>
              Promise.resolve({
                users: mockUsers.users,
                total: 100,
                offset: 0,
                limit: 50,
              }),
          });
        }
        return Promise.resolve({ ok: true, json: () => Promise.resolve({}) });
      });

      render(<AdminUserManagement />);

      await waitFor(() => {
        const prevButton = screen.getByText('Previous');
        expect(prevButton).toBeDisabled();
      });
    });
  });

  describe('error handling', () => {
    it('displays error message on API failure', async () => {
      mockFetch.mockImplementation(() =>
        Promise.resolve({
          ok: false,
          json: () => Promise.resolve({ error: 'Permission denied' }),
        })
      );

      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByText('Permission denied')).toBeInTheDocument();
      });
    });
  });

  describe('loading state', () => {
    it('shows loading message when users are being fetched', async () => {
      mockFetch.mockImplementation(
        () =>
          new Promise((resolve) => {
            setTimeout(() => {
              resolve({
                ok: true,
                json: () => Promise.resolve(mockUsers),
              });
            }, 100);
          })
      );

      render(<AdminUserManagement />);

      expect(screen.getByText('Loading users...')).toBeInTheDocument();

      await waitFor(() => {
        expect(screen.queryByText('Loading users...')).not.toBeInTheDocument();
      });
    });
  });

  describe('accessibility', () => {
    it('has proper aria labels for checkboxes', async () => {
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByRole('checkbox', { name: /select all/i })).toBeInTheDocument();
        expect(screen.getByRole('checkbox', { name: /select admin@example.com/i })).toBeInTheDocument();
      });
    });

    it('has proper aria labels for role dropdowns', async () => {
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByRole('combobox', { name: /role for admin@example.com/i })).toBeInTheDocument();
      });
    });

    it('has proper table structure', async () => {
      render(<AdminUserManagement />);

      await waitFor(() => {
        expect(screen.getByRole('grid')).toBeInTheDocument();
      });
    });
  });
});
