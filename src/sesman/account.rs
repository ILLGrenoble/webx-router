use std::fmt;

use nix::unistd::User;
use users::get_user_groups;

/// The `Account` struct represents a user account in the system.
#[derive(Clone)]
pub struct Account {
    username: String,
    home: String,
    uid: u32,
    gid: u32,
    groups: Vec<u32>
}

impl Account {
    /// Creates a new `Account` instance.
    ///
    /// # Arguments
    /// * `username` - The username of the account.
    /// * `home` - The home directory of the account.
    /// * `uid` - The user ID of the account.
    /// * `gid` - The group ID of the account.
    /// * `groups` - The list of group IDs the account belongs to.
    ///
    /// # Returns
    /// A new `Account` instance.
    pub fn new(username: &str, home: &str, uid: u32, gid: u32, groups: Vec<u32>) -> Self {
        Account {
            username: username.into(),
            home: home.into(),
            uid,
            gid,
            groups
        }
    }

    /// Returns the username of the account.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Returns the home directory of the account.
    pub fn home(&self) -> &str {
        &self.home
    }

    /// Returns the user ID of the account.
    pub fn uid(&self) -> u32 {
        self.uid
    }

    /// Returns the group ID of the account.
    pub fn gid(&self) -> u32 {
        self.gid
    }

    /// Returns the list of group IDs the account belongs to.
    pub fn groups(&self) -> &[u32] {
        &self.groups
    }

    /// Creates an `Account` instance from a `User`.
    ///
    /// # Arguments
    /// * `user` - The `User` to convert.
    ///
    /// # Returns
    /// An `Option` containing the `Account` or `None` if the conversion fails.
    pub fn from_user(user: User) -> Option<Account> {
        let uid = user.uid.as_raw();
        let gid = user.gid.as_raw();
        let username = user.name.as_str();
        if let Some(home) = user.dir.to_str() {
            let groups: Vec<u32> = get_user_groups(username, gid)
            .unwrap_or_default()
            .iter()
            .filter(|group| {
                // only return the root group if the user is the root user
                if uid == 0 {
                    return true;
                }
                group.gid() > 0
            })
            .map(|group| group.gid())
            .collect();

            let account = Account::new(username, home, uid, gid, groups);
            return Some(account);
        }

        None
    }
}

impl fmt::Display for Account {
    /// Formats the `Account` for display.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "username = {}, home = {} uid = {}, gid = {}, groups = {:?}", self.username, self.home, self.uid, self.gid, &self.groups)
    }
}
