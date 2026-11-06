//! The server's public types. Consumers select `default-features = false,
//! features = ["contract"]`, which builds without a runtime dependency.

mod calls;
pub use calls::LoreProvider;
mod files;
mod ids;
mod permissions;
mod resource;

pub use files::{
    Cover, Covers, CoversRequest, DEFAULT_BRANCH, EntryType, FileEntry, FileListing, FileRef,
    FileThumbnail, FileThumbnails, FolderRef, ListFilesRequest, MOST_AT_ONCE, ReadyThumbnail,
    RepositorySummary, ThumbnailRequest, ThumbnailSize, ThumbnailsRequest,
};
pub use ids::{BranchName, FilePath, LoreIdError, RepositoryId};
pub use permissions::{LorePermission, LoreScope};
pub use resource::{LoreDocument, LoreResource, LoreResourceError};
