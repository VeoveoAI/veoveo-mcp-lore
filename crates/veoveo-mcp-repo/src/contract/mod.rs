//! The server's public types. Consumers select `default-features = false,
//! features = ["contract"]`, which builds without a runtime dependency.

mod activity;
mod calls;
pub use calls::RepoProvider;
mod files;
mod ids;
mod permissions;
mod resource;

pub use activity::{
    Activity, ActivityRequest, BranchRow, Change, ChangeAction, ChangeKind, Changes,
    ChangesRequest, FileDiff, FileDiffRequest, LinkChange, LinkPushes, LinkPushesRequest,
    MOST_SUMMARIES, MergedFrom, Push, PushRef, PushedBy, Running, Summaries, SummariesRequest,
    Summary,
};
pub use files::{
    Browse, BrowseItem, BrowseRequest, Cover, Covers, CoversRequest, DEFAULT_BRANCH, EntryType,
    FileEntry, FileListing, FileRef, FileThumbnail, FileThumbnails, FolderRef, ListFilesRequest,
    MOST_AT_ONCE, MOST_ON_A_PAGE, PagePicture, ReadyThumbnail, RepositorySummary, ThumbnailRequest,
    ThumbnailSize, ThumbnailsRequest,
};
pub use ids::{BranchName, FilePath, RepoIdError, RepositoryId, Revision};
pub use permissions::{RepoPermission, RepoScope};
pub use resource::{RepoDocument, RepoResource, RepoResourceError};
