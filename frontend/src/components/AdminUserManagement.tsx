import { useCallback, useEffect, useState } from 'react';
import {
  useAdminUsers,
  type AdminUser,
  type UserRole,
} from '../hooks/useAdminUsers';

export interface AdminUserManagementProps {
  baseUrl?: string;
  token?: string;
  currentUserId?: string;
}

const ROLE_DISPLAY_NAMES: Record<UserRole, string> = {
  admin: 'Admin',
  developer: 'Developer',
  viewer: 'Viewer',
};

const ROLES: UserRole[] = ['admin', 'developer', 'viewer'];

function formatDate(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

export function AdminUserManagement({
  baseUrl = '',
  token,
  currentUserId,
}: AdminUserManagementProps) {
  const {
    users,
    total,
    loading,
    error,
    offset,
    limit,
    searchQuery,
    setSearchQuery,
    setPage,
    updateUserRole,
    deleteUser,
    bulkUpdateRoles,
  } = useAdminUsers({ baseUrl, token, autoFetch: true });

  // Local state for UI interactions
  const [selectedUsers, setSelectedUsers] = useState<Set<string>>(new Set());
  const [bulkRole, setBulkRole] = useState<UserRole>('viewer');
  const [searchInput, setSearchInput] = useState('');
  const [confirmingDelete, setConfirmingDelete] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [roleUpdating, setRoleUpdating] = useState<string | null>(null);

  // Calculate pagination
  const currentPage = Math.floor(offset / limit);
  const totalPages = Math.ceil(total / limit);

  // Clear success message after timeout
  useEffect(() => {
    if (successMessage) {
      const timer = setTimeout(() => setSuccessMessage(null), 3000);
      return () => clearTimeout(timer);
    }
  }, [successMessage]);

  // Handle search with debounce
  useEffect(() => {
    const timer = setTimeout(() => {
      if (searchInput !== searchQuery) {
        setSearchQuery(searchInput);
        setPage(0); // Reset to first page on search
      }
    }, 300);
    return () => clearTimeout(timer);
  }, [searchInput, searchQuery, setSearchQuery, setPage]);

  const handleSearchChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setSearchInput(e.target.value);
    },
    []
  );

  const handleSelectUser = useCallback((userId: string, checked: boolean) => {
    setSelectedUsers((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(userId);
      } else {
        next.delete(userId);
      }
      return next;
    });
  }, []);

  const handleSelectAll = useCallback(
    (checked: boolean) => {
      if (checked) {
        const allIds = users
          .filter((u) => u.id !== currentUserId) // Don't select current user
          .map((u) => u.id);
        setSelectedUsers(new Set(allIds));
      } else {
        setSelectedUsers(new Set());
      }
    },
    [users, currentUserId]
  );

  const handleRoleChange = useCallback(
    async (userId: string, newRole: UserRole) => {
      setRoleUpdating(userId);
      const success = await updateUserRole(userId, newRole);
      setRoleUpdating(null);
      if (success) {
        setSuccessMessage(`Role updated to ${ROLE_DISPLAY_NAMES[newRole]}`);
      }
    },
    [updateUserRole]
  );

  const handleDeleteUser = useCallback(
    async (userId: string) => {
      if (confirmingDelete !== userId) {
        setConfirmingDelete(userId);
        return;
      }

      const success = await deleteUser(userId);
      setConfirmingDelete(null);
      if (success) {
        setSelectedUsers((prev) => {
          const next = new Set(prev);
          next.delete(userId);
          return next;
        });
        setSuccessMessage('User deleted successfully');
      }
    },
    [confirmingDelete, deleteUser]
  );

  const handleCancelDelete = useCallback(() => {
    setConfirmingDelete(null);
  }, []);

  const handleBulkRoleChange = useCallback(async () => {
    const userIds = Array.from(selectedUsers);
    if (userIds.length === 0) return;

    const result = await bulkUpdateRoles(userIds, bulkRole);
    if (result) {
      setSuccessMessage(
        `Updated ${result.success_count} user${result.success_count !== 1 ? 's' : ''} to ${ROLE_DISPLAY_NAMES[bulkRole]}`
      );
      setSelectedUsers(new Set());
    }
  }, [selectedUsers, bulkRole, bulkUpdateRoles]);

  const handlePageChange = useCallback(
    (newPage: number) => {
      setPage(newPage);
      setSelectedUsers(new Set()); // Clear selection on page change
    },
    [setPage]
  );

  const allSelectableSelected =
    users.length > 0 &&
    users.filter((u) => u.id !== currentUserId).every((u) => selectedUsers.has(u.id));

  return (
    <div className="admin-user-management">
      <h2 className="admin-user-management__title">User Management</h2>

      {error && (
        <div className="admin-user-management__error" role="alert">
          <span className="admin-user-management__error-icon" aria-hidden="true">
            Error
          </span>
          <span className="admin-user-management__error-message">{error}</span>
        </div>
      )}

      {successMessage && (
        <div
          className="admin-user-management__success"
          role="status"
          aria-live="polite"
        >
          {successMessage}
        </div>
      )}

      {/* Search and Bulk Actions Bar */}
      <div className="admin-user-management__toolbar">
        <div className="admin-user-management__search">
          <label htmlFor="user-search" className="admin-user-management__search-label">
            Search users
          </label>
          <input
            type="text"
            id="user-search"
            value={searchInput}
            onChange={handleSearchChange}
            placeholder="Search by email..."
            className="admin-user-management__search-input"
            aria-describedby="search-hint"
          />
          <span id="search-hint" className="admin-user-management__search-hint">
            {total} user{total !== 1 ? 's' : ''} found
          </span>
        </div>

        {selectedUsers.size > 0 && (
          <div className="admin-user-management__bulk-actions">
            <span className="admin-user-management__selected-count">
              {selectedUsers.size} selected
            </span>
            <select
              value={bulkRole}
              onChange={(e) => setBulkRole(e.target.value as UserRole)}
              className="admin-user-management__bulk-select"
              aria-label="Bulk role assignment"
            >
              {ROLES.map((role) => (
                <option key={role} value={role}>
                  {ROLE_DISPLAY_NAMES[role]}
                </option>
              ))}
            </select>
            <button
              type="button"
              onClick={handleBulkRoleChange}
              disabled={loading}
              className="admin-user-management__button admin-user-management__button--primary"
            >
              Apply Role
            </button>
          </div>
        )}
      </div>

      {/* User List Table */}
      <div className="admin-user-management__table-container">
        <table className="admin-user-management__table" role="grid">
          <thead>
            <tr>
              <th scope="col" className="admin-user-management__th admin-user-management__th--checkbox">
                <label className="admin-user-management__checkbox-wrapper">
                  <input
                    type="checkbox"
                    checked={allSelectableSelected}
                    onChange={(e) => handleSelectAll(e.target.checked)}
                    aria-label="Select all users"
                    className="admin-user-management__checkbox"
                  />
                </label>
              </th>
              <th scope="col" className="admin-user-management__th">Email</th>
              <th scope="col" className="admin-user-management__th">Role</th>
              <th scope="col" className="admin-user-management__th">Created</th>
              <th scope="col" className="admin-user-management__th">Actions</th>
            </tr>
          </thead>
          <tbody>
            {loading && users.length === 0 ? (
              <tr>
                <td colSpan={5} className="admin-user-management__loading">
                  Loading users...
                </td>
              </tr>
            ) : users.length === 0 ? (
              <tr>
                <td colSpan={5} className="admin-user-management__empty">
                  {searchQuery ? 'No users match your search' : 'No users found'}
                </td>
              </tr>
            ) : (
              users.map((user) => (
                <UserRow
                  key={user.id}
                  user={user}
                  isSelected={selectedUsers.has(user.id)}
                  isCurrentUser={user.id === currentUserId}
                  isConfirmingDelete={confirmingDelete === user.id}
                  isUpdatingRole={roleUpdating === user.id}
                  onSelect={handleSelectUser}
                  onRoleChange={handleRoleChange}
                  onDelete={handleDeleteUser}
                  onCancelDelete={handleCancelDelete}
                />
              ))
            )}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <nav
          className="admin-user-management__pagination"
          aria-label="User list pagination"
        >
          <button
            type="button"
            onClick={() => handlePageChange(currentPage - 1)}
            disabled={currentPage === 0 || loading}
            className="admin-user-management__page-button"
            aria-label="Previous page"
          >
            Previous
          </button>
          <span className="admin-user-management__page-info">
            Page {currentPage + 1} of {totalPages}
          </span>
          <button
            type="button"
            onClick={() => handlePageChange(currentPage + 1)}
            disabled={currentPage >= totalPages - 1 || loading}
            className="admin-user-management__page-button"
            aria-label="Next page"
          >
            Next
          </button>
        </nav>
      )}
    </div>
  );
}

interface UserRowProps {
  user: AdminUser;
  isSelected: boolean;
  isCurrentUser: boolean;
  isConfirmingDelete: boolean;
  isUpdatingRole: boolean;
  onSelect: (userId: string, checked: boolean) => void;
  onRoleChange: (userId: string, newRole: UserRole) => void;
  onDelete: (userId: string) => void;
  onCancelDelete: () => void;
}

function UserRow({
  user,
  isSelected,
  isCurrentUser,
  isConfirmingDelete,
  isUpdatingRole,
  onSelect,
  onRoleChange,
  onDelete,
  onCancelDelete,
}: UserRowProps) {
  return (
    <tr
      className={`admin-user-management__row ${
        isSelected ? 'admin-user-management__row--selected' : ''
      } ${isCurrentUser ? 'admin-user-management__row--current' : ''}`}
    >
      <td className="admin-user-management__td admin-user-management__td--checkbox">
        <label className="admin-user-management__checkbox-wrapper">
          <input
            type="checkbox"
            checked={isSelected}
            onChange={(e) => onSelect(user.id, e.target.checked)}
            disabled={isCurrentUser}
            aria-label={`Select ${user.email}`}
            className="admin-user-management__checkbox"
          />
        </label>
      </td>
      <td className="admin-user-management__td admin-user-management__td--email">
        {user.email}
        {isCurrentUser && (
          <span className="admin-user-management__current-badge">(you)</span>
        )}
      </td>
      <td className="admin-user-management__td admin-user-management__td--role">
        <select
          value={user.role}
          onChange={(e) => onRoleChange(user.id, e.target.value as UserRole)}
          disabled={isCurrentUser || isUpdatingRole}
          className={`admin-user-management__role-select admin-user-management__role-select--${user.role}`}
          aria-label={`Role for ${user.email}`}
        >
          {ROLES.map((role) => (
            <option key={role} value={role}>
              {ROLE_DISPLAY_NAMES[role]}
            </option>
          ))}
        </select>
        {isUpdatingRole && (
          <span className="admin-user-management__updating">Updating...</span>
        )}
      </td>
      <td className="admin-user-management__td admin-user-management__td--date">
        {formatDate(user.created_at)}
      </td>
      <td className="admin-user-management__td admin-user-management__td--actions">
        {isCurrentUser ? (
          <span className="admin-user-management__no-action">-</span>
        ) : isConfirmingDelete ? (
          <div className="admin-user-management__delete-confirm">
            <span>Delete?</span>
            <button
              type="button"
              onClick={() => onDelete(user.id)}
              className="admin-user-management__button admin-user-management__button--danger"
              aria-label={`Confirm delete ${user.email}`}
            >
              Yes
            </button>
            <button
              type="button"
              onClick={onCancelDelete}
              className="admin-user-management__button admin-user-management__button--secondary"
              aria-label="Cancel delete"
            >
              No
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={() => onDelete(user.id)}
            className="admin-user-management__button admin-user-management__button--danger-outline"
            aria-label={`Delete ${user.email}`}
          >
            Delete
          </button>
        )}
      </td>
    </tr>
  );
}
