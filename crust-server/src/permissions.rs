/// Permission checking logic for repositories and organizations.
///
/// This module handles authorization checks for CRUST resources.
/// Permission hierarchy:
/// - owner: Full access to repo (create, read, write, delete)
/// - write: Can push commits, create refs
/// - read: Can fetch, read objects, view metadata
/// - none: No access (unless repo is public for read)
use uuid::Uuid;

/// Permission levels for repository access
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    None = 0,
    Read = 1,
    Write = 2,
    Owner = 3,
}

impl Permission {
    /// Check if this permission allows reading
    pub fn can_read(&self) -> bool {
        matches!(
            self,
            Permission::Read | Permission::Write | Permission::Owner
        )
    }

    /// Check if this permission allows writing
    pub fn can_write(&self) -> bool {
        matches!(self, Permission::Write | Permission::Owner)
    }

    /// Check if this permission allows ownership actions
    pub fn is_owner(&self) -> bool {
        matches!(self, Permission::Owner)
    }
}

/// Permission context for checking access
pub struct PermissionContext {
    pub user_id: Uuid,
    pub repo_owner_id: Uuid,
    pub repo_is_public: bool,
}

impl PermissionContext {
    pub fn new(user_id: Uuid, repo_owner_id: Uuid, repo_is_public: bool) -> Self {
        Self {
            user_id,
            repo_owner_id,
            repo_is_public,
        }
    }

    /// Determine permission level for user accessing a repository
    ///
    /// Rules:
    /// 1. If user is owner, permission is Owner
    /// 2. If repo is public, unauthenticated users get Read
    /// 3. Otherwise, no permission (must be explicitly granted)
    /// 4. In production, check repo_permissions table for explicit grants
    pub fn get_permission(&self) -> Permission {
        // Owner always has full access
        if self.user_id == self.repo_owner_id {
            return Permission::Owner;
        }

        // Public repos allow read access to everyone
        if self.repo_is_public {
            return Permission::Read;
        }

        // Private repos require explicit permission (not implemented yet)
        Permission::None
    }

    /// Check if user can read the repository
    pub fn can_read(&self) -> bool {
        self.get_permission().can_read()
    }

    /// Check if user can write to the repository
    pub fn can_write(&self) -> bool {
        self.get_permission().can_write()
    }

    /// Check if user is the owner
    pub fn is_owner(&self) -> bool {
        self.get_permission().is_owner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owner_permission() {
        let user_id = Uuid::new_v4();
        let ctx = PermissionContext::new(user_id, user_id, false);
        assert!(ctx.is_owner());
        assert!(ctx.can_write());
        assert!(ctx.can_read());
    }

    #[test]
    fn test_public_repo_read_access() {
        let user_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let ctx = PermissionContext::new(user_id, owner_id, true);
        assert!(!ctx.is_owner());
        assert!(!ctx.can_write());
        assert!(ctx.can_read());
    }

    #[test]
    fn test_private_repo_no_access() {
        let user_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let ctx = PermissionContext::new(user_id, owner_id, false);
        assert!(!ctx.is_owner());
        assert!(!ctx.can_write());
        assert!(!ctx.can_read());
    }

    #[test]
    fn test_permission_ordering() {
        assert!(Permission::Owner > Permission::Write);
        assert!(Permission::Write > Permission::Read);
        assert!(Permission::Read > Permission::None);
    }
}
