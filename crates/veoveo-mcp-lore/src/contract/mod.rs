//! The server's public types. Consumers select `default-features = false,
//! features = ["contract"]`, which builds without a runtime dependency.

mod calls;
pub use calls::LoreProvider;
mod files;
mod ids;
mod permissions;
mod resource;

pub use files::{
    DEFAULT_BRANCH, EntryType, FileEntry, FileListing, FileThumbnail, ListFilesRequest,
    RepositorySummary, ThumbnailRequest, ThumbnailSize,
};
pub use ids::{BranchName, FilePath, LoreIdError, RepositoryId};
pub use permissions::{LorePermission, LoreScope};
pub use resource::{LoreDocument, LoreResource, LoreResourceError};
