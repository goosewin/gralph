import { useCallback, useEffect, useState } from 'react';
import { useUserSettings } from '../hooks/useUserSettings';

export interface UserSettingsProps {
  baseUrl?: string;
  token?: string;
}

interface FormErrors {
  name?: string;
  email?: string;
  oldPassword?: string;
  newPassword?: string;
  confirmPassword?: string;
}

const PROVIDER_DISPLAY_NAMES: Record<string, string> = {
  github: 'GitHub',
  google: 'Google',
  gitlab: 'GitLab',
};

const PROVIDER_ICONS: Record<string, string> = {
  github: '🐙',
  google: '🔍',
  gitlab: '🦊',
};

function formatDate(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

export function UserSettings({ baseUrl = '', token }: UserSettingsProps) {
  const {
    profile,
    linkedAccounts,
    loading,
    error,
    updateProfile,
    changePassword,
    unlinkAccount,
  } = useUserSettings({ baseUrl, token });

  // Profile form state
  const [profileFormData, setProfileFormData] = useState({
    name: '',
    email: '',
  });
  const [profileSuccess, setProfileSuccess] = useState(false);

  // Password form state
  const [passwordFormData, setPasswordFormData] = useState({
    oldPassword: '',
    newPassword: '',
    confirmPassword: '',
  });
  const [passwordSuccess, setPasswordSuccess] = useState(false);
  const [showPasswords, setShowPasswords] = useState(false);

  // Form validation errors
  const [formErrors, setFormErrors] = useState<FormErrors>({});

  // Unlink confirmation
  const [unlinkingProvider, setUnlinkingProvider] = useState<string | null>(null);

  // Update form data when profile loads
  useEffect(() => {
    if (profile) {
      setProfileFormData({
        name: profile.name || '',
        email: profile.email,
      });
    }
  }, [profile]);

  // Clear success messages after timeout
  useEffect(() => {
    if (profileSuccess) {
      const timer = setTimeout(() => setProfileSuccess(false), 3000);
      return () => clearTimeout(timer);
    }
  }, [profileSuccess]);

  useEffect(() => {
    if (passwordSuccess) {
      const timer = setTimeout(() => setPasswordSuccess(false), 3000);
      return () => clearTimeout(timer);
    }
  }, [passwordSuccess]);

  const validateProfileForm = useCallback((data: typeof profileFormData): boolean => {
    const errors: FormErrors = {};

    if (data.email && !isValidEmail(data.email)) {
      errors.email = 'Invalid email address';
    }

    setFormErrors(errors);
    return Object.keys(errors).length === 0;
  }, []);

  const validatePasswordForm = useCallback((): boolean => {
    const errors: FormErrors = {};

    if (!passwordFormData.oldPassword) {
      errors.oldPassword = 'Current password is required';
    }

    if (!passwordFormData.newPassword) {
      errors.newPassword = 'New password is required';
    } else if (passwordFormData.newPassword.length < 8) {
      errors.newPassword = 'Password must be at least 8 characters';
    }

    if (passwordFormData.newPassword !== passwordFormData.confirmPassword) {
      errors.confirmPassword = 'Passwords do not match';
    }

    setFormErrors(errors);
    return Object.keys(errors).length === 0;
  }, [passwordFormData]);

  const handleProfileSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setProfileSuccess(false);

    // Read values directly from form to avoid stale state
    const formData = new FormData(e.currentTarget);
    const currentData = {
      name: (formData.get('name') as string) || '',
      email: (formData.get('email') as string) || '',
    };


    if (!validateProfileForm(currentData)) {
      return;
    }

    const success = await updateProfile({
      name: currentData.name || undefined,
      email: currentData.email || undefined,
    });

    if (success) {
      setProfileSuccess(true);
    }
  };

  const handlePasswordSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      setPasswordSuccess(false);

      if (!validatePasswordForm()) {
        return;
      }

      const success = await changePassword({
        old_password: passwordFormData.oldPassword,
        new_password: passwordFormData.newPassword,
      });

      if (success) {
        setPasswordSuccess(true);
        setPasswordFormData({
          oldPassword: '',
          newPassword: '',
          confirmPassword: '',
        });
      }
    },
    [passwordFormData, changePassword, validatePasswordForm]
  );

  const handleUnlinkAccount = useCallback(
    async (provider: string) => {
      if (unlinkingProvider !== provider) {
        setUnlinkingProvider(provider);
        return;
      }

      const success = await unlinkAccount(provider);
      if (success) {
        setUnlinkingProvider(null);
      }
    },
    [unlinkAccount, unlinkingProvider]
  );

  const handleCancelUnlink = useCallback(() => {
    setUnlinkingProvider(null);
  }, []);

  const handleProfileInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const { name, value } = e.target;
      setProfileFormData((prev) => ({ ...prev, [name]: value }));
      // Clear error for this field
      setFormErrors((prev) => ({ ...prev, [name]: undefined }));
    },
    []
  );

  const handlePasswordInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const { name, value } = e.target;
      setPasswordFormData((prev) => ({ ...prev, [name]: value }));
      // Clear error for this field
      setFormErrors((prev) => ({ ...prev, [name]: undefined }));
    },
    []
  );

  const toggleShowPasswords = useCallback(() => {
    setShowPasswords((prev) => !prev);
  }, []);

  return (
    <div className="user-settings">
      <h2 className="user-settings__title">User Settings</h2>

      {error && (
        <div className="user-settings__error" role="alert">
          <span className="user-settings__error-icon" aria-hidden="true">
            ⚠️
          </span>
          <span className="user-settings__error-message">{error}</span>
        </div>
      )}

      {/* Profile Section */}
      <section
        className="user-settings__section"
        aria-labelledby="profile-section-title"
      >
        <h3 id="profile-section-title" className="user-settings__section-title">
          Profile
        </h3>
        <p className="user-settings__section-description">
          Update your personal information.
        </p>

        {profileSuccess && (
          <div
            className="user-settings__success"
            role="status"
            aria-live="polite"
          >
            Profile updated successfully!
          </div>
        )}

        <form onSubmit={handleProfileSubmit} className="user-settings__form">
          <div className="user-settings__form-group">
            <label
              htmlFor="profile-name"
              className="user-settings__form-label"
            >
              Name
            </label>
            <input
              type="text"
              id="profile-name"
              name="name"
              value={profileFormData.name}
              onChange={handleProfileInputChange}
              className={`user-settings__form-input ${
                formErrors.name ? 'user-settings__form-input--error' : ''
              }`}
              placeholder="Your name"
              aria-describedby={formErrors.name ? 'name-error' : undefined}
            />
            {formErrors.name && (
              <span id="name-error" className="user-settings__form-error">
                {formErrors.name}
              </span>
            )}
          </div>

          <div className="user-settings__form-group">
            <label
              htmlFor="profile-email"
              className="user-settings__form-label"
            >
              Email
            </label>
            <input
              type="email"
              id="profile-email"
              name="email"
              value={profileFormData.email}
              onChange={handleProfileInputChange}
              className={`user-settings__form-input ${
                formErrors.email ? 'user-settings__form-input--error' : ''
              }`}
              placeholder="your.email@example.com"
              aria-describedby={formErrors.email ? 'email-error' : undefined}
            />
            {formErrors.email && (
              <span id="email-error" className="user-settings__form-error">
                {formErrors.email}
              </span>
            )}
          </div>

          {profile && (
            <div className="user-settings__form-group">
              <label className="user-settings__form-label">Role</label>
              <div className="user-settings__form-value">
                <span className={`user-settings__role user-settings__role--${profile.role}`}>
                  {profile.role}
                </span>
              </div>
            </div>
          )}

          <div className="user-settings__form-actions">
            <button
              type="submit"
              className="user-settings__button user-settings__button--primary"
              disabled={loading}
            >
              {loading ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        </form>
      </section>

      {/* Password Section */}
      <section
        className="user-settings__section"
        aria-labelledby="password-section-title"
      >
        <h3 id="password-section-title" className="user-settings__section-title">
          Change Password
        </h3>
        <p className="user-settings__section-description">
          Update your password. You will need to enter your current password.
        </p>

        {passwordSuccess && (
          <div
            className="user-settings__success"
            role="status"
            aria-live="polite"
          >
            Password changed successfully!
          </div>
        )}

        <form onSubmit={handlePasswordSubmit} className="user-settings__form">
          <div className="user-settings__form-group">
            <label
              htmlFor="old-password"
              className="user-settings__form-label"
            >
              Current Password
            </label>
            <div className="user-settings__password-wrapper">
              <input
                type={showPasswords ? 'text' : 'password'}
                id="old-password"
                name="oldPassword"
                value={passwordFormData.oldPassword}
                onChange={handlePasswordInputChange}
                className={`user-settings__form-input ${
                  formErrors.oldPassword ? 'user-settings__form-input--error' : ''
                }`}
                placeholder="Enter current password"
                aria-describedby={
                  formErrors.oldPassword ? 'old-password-error' : undefined
                }
                autoComplete="current-password"
              />
            </div>
            {formErrors.oldPassword && (
              <span id="old-password-error" className="user-settings__form-error">
                {formErrors.oldPassword}
              </span>
            )}
          </div>

          <div className="user-settings__form-group">
            <label
              htmlFor="new-password"
              className="user-settings__form-label"
            >
              New Password
            </label>
            <div className="user-settings__password-wrapper">
              <input
                type={showPasswords ? 'text' : 'password'}
                id="new-password"
                name="newPassword"
                value={passwordFormData.newPassword}
                onChange={handlePasswordInputChange}
                className={`user-settings__form-input ${
                  formErrors.newPassword ? 'user-settings__form-input--error' : ''
                }`}
                placeholder="Enter new password"
                aria-describedby={
                  formErrors.newPassword ? 'new-password-error' : undefined
                }
                autoComplete="new-password"
              />
            </div>
            {formErrors.newPassword && (
              <span id="new-password-error" className="user-settings__form-error">
                {formErrors.newPassword}
              </span>
            )}
          </div>

          <div className="user-settings__form-group">
            <label
              htmlFor="confirm-password"
              className="user-settings__form-label"
            >
              Confirm New Password
            </label>
            <div className="user-settings__password-wrapper">
              <input
                type={showPasswords ? 'text' : 'password'}
                id="confirm-password"
                name="confirmPassword"
                value={passwordFormData.confirmPassword}
                onChange={handlePasswordInputChange}
                className={`user-settings__form-input ${
                  formErrors.confirmPassword ? 'user-settings__form-input--error' : ''
                }`}
                placeholder="Confirm new password"
                aria-describedby={
                  formErrors.confirmPassword ? 'confirm-password-error' : undefined
                }
                autoComplete="new-password"
              />
            </div>
            {formErrors.confirmPassword && (
              <span id="confirm-password-error" className="user-settings__form-error">
                {formErrors.confirmPassword}
              </span>
            )}
          </div>

          <div className="user-settings__form-group">
            <label className="user-settings__checkbox-label">
              <input
                type="checkbox"
                checked={showPasswords}
                onChange={toggleShowPasswords}
                className="user-settings__checkbox"
              />
              <span>Show passwords</span>
            </label>
          </div>

          <div className="user-settings__form-actions">
            <button
              type="submit"
              className="user-settings__button user-settings__button--primary"
              disabled={loading}
            >
              {loading ? 'Changing...' : 'Change Password'}
            </button>
          </div>
        </form>
      </section>

      {/* Linked Accounts Section */}
      <section
        className="user-settings__section"
        aria-labelledby="linked-accounts-section-title"
      >
        <h3 id="linked-accounts-section-title" className="user-settings__section-title">
          Linked Accounts
        </h3>
        <p className="user-settings__section-description">
          Manage your connected social accounts for quick sign-in.
        </p>

        {linkedAccounts.length === 0 ? (
          <div className="user-settings__empty">
            <p>No linked accounts.</p>
            <p className="user-settings__empty-hint">
              Link your social accounts for faster sign-in.
            </p>
          </div>
        ) : (
          <ul className="user-settings__linked-accounts" role="list">
            {linkedAccounts.map((account) => (
              <li key={account.provider} className="user-settings__linked-account">
                <div className="user-settings__linked-account-info">
                  <span
                    className="user-settings__linked-account-icon"
                    aria-hidden="true"
                  >
                    {PROVIDER_ICONS[account.provider] || '🔗'}
                  </span>
                  <div className="user-settings__linked-account-details">
                    <span className="user-settings__linked-account-provider">
                      {PROVIDER_DISPLAY_NAMES[account.provider] || account.provider}
                    </span>
                    <span className="user-settings__linked-account-email">
                      {account.username || account.email}
                    </span>
                    <span className="user-settings__linked-account-date">
                      Linked {formatDate(account.linked_at)}
                    </span>
                  </div>
                </div>
                <div className="user-settings__linked-account-actions">
                  {unlinkingProvider === account.provider ? (
                    <>
                      <span className="user-settings__unlink-confirm-text">
                        Unlink this account?
                      </span>
                      <button
                        type="button"
                        className="user-settings__button user-settings__button--danger"
                        onClick={() => handleUnlinkAccount(account.provider)}
                        disabled={loading}
                        aria-label={`Confirm unlink ${PROVIDER_DISPLAY_NAMES[account.provider] || account.provider}`}
                      >
                        Confirm
                      </button>
                      <button
                        type="button"
                        className="user-settings__button user-settings__button--secondary"
                        onClick={handleCancelUnlink}
                        aria-label="Cancel unlink"
                      >
                        Cancel
                      </button>
                    </>
                  ) : (
                    <button
                      type="button"
                      className="user-settings__button user-settings__button--secondary"
                      onClick={() => handleUnlinkAccount(account.provider)}
                      disabled={loading}
                      aria-label={`Unlink ${PROVIDER_DISPLAY_NAMES[account.provider] || account.provider}`}
                    >
                      Unlink
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

function isValidEmail(email: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}
