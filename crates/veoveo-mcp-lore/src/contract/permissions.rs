//! The server's OAuth scope and permissions. Each permission names one action on one
//! resource type, with the access kind and risks the `ai.veoveo/authorization`
//! declaration carries; installation roles grant them and the Hub forwards the
//! granted ones.
use veoveo_hub_sdk::registration::{Access, PermissionDefinition, Risk};
use veoveo_types::{PermissionName, Vocabulary};

/// The OAuth scope that names the server's namespace. Scopes are coarse; the
/// permissions carry fine-grained access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Vocabulary)]
#[vocabulary(scope)]
pub enum LoreScope {
    #[vocabulary(rename = "lore")]
    Files,
}

/// The server's permissions.
/// ```compile_fail
/// use veoveo_mcp_lore::contract::LorePermission;
/// fn requires(_: LorePermission) {}
/// requires("lore.files.read");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Vocabulary)]
pub enum LorePermission {
    /// List what the repositories you can read hold and see pictures of their files.
    #[vocabulary(rename = "lore.files.read")]
    FilesRead,
}

impl LorePermission {
    /// The permission's wire name, as declared and as granted.
    pub fn name(self) -> PermissionName {
        PermissionName::parse(self.as_str()).expect("declared lore permission spelling")
    }
}

impl PermissionDefinition for LorePermission {
    fn resource(self) -> &'static str {
        "lore.file"
    }

    fn access(self) -> Access {
        Access::Read
    }

    fn risks(self) -> &'static [Risk] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_follow_the_naming_form() {
        for permission in LorePermission::ALL {
            let name = permission.as_str();
            let mut parts = name.split('.');
            assert_eq!(parts.next(), Some("lore"), "{name}");
            assert!(parts.next().is_some_and(|plural| !plural.is_empty()));
            assert!(parts.next().is_some_and(|verb| !verb.is_empty()));
            assert_eq!(parts.next(), None, "{name}");
            assert_eq!(permission.name().as_str(), name);
        }
        assert_eq!(LoreScope::Files.as_str(), "lore");
    }
}
