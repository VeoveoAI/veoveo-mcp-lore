//! The tools: what a repository holds, what shows a folder, and pictures of its files.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{BranchName, FilePath, LoreProvider, RepositoryId};

/// The branch `list_files` reads when none is named.
pub const DEFAULT_BRANCH: &str = "main";

/// The arguments of `list_files`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = LoreProvider, output = FileListing, name = "list_files")]
pub struct ListFilesRequest {
    /// The repository to list. Without one, lists the repositories you can read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryId>,
    /// The branch to read; `main` by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchName>,
    /// The folder to list; the repository's top by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<FilePath>,
}

/// A repository the caller can read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositorySummary {
    pub id: RepositoryId,
    pub name: String,
}

/// What a name in a folder is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    File,
    Directory,
    Link,
}

/// One name in a folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: EntryType,
    /// The bytes of a file or of all a folder holds, as the Hub lists them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// What `list_files` answers: the repositories, or a folder of one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct FileListing {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repositories: Vec<RepositorySummary>,
    /// The repository `entries` belong to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryId>,
    /// The branch `entries` were read from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchName>,
    /// The folder `entries` are in; none is the repository's top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<FilePath>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<FileEntry>,
}

/// How large a picture to get.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ThumbnailSize {
    #[default]
    Small,
    Large,
}

/// The arguments of `thumbnail`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = LoreProvider, output = FileThumbnail, name = "thumbnail")]
pub struct ThumbnailRequest {
    pub repository: RepositoryId,
    pub branch: BranchName,
    pub path: FilePath,
    #[serde(default)]
    pub size: ThumbnailSize,
}

/// The most folders one `covers` names, and files one `thumbnails` names.
pub const MOST_AT_ONCE: usize = 64;
/// The most items one `browse` answers.
pub const MOST_ON_A_PAGE: u32 = 120;

/// The arguments of `browse`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = LoreProvider, output = Browse, name = "browse")]
pub struct BrowseRequest {
    /// The repository to browse. Without one, the repositories you can read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryId>,
    /// The branch to read; `main` by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchName>,
    /// The folder; the repository's top by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<FilePath>,
    /// Find names that hold this, at any depth under the folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    /// The `next` of the page before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// How many items, at most 120.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// One name on a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowseItem {
    pub name: String,
    /// The repository, for a page of repositories; otherwise the path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<FilePath>,
    #[serde(rename = "type")]
    pub entry_type: EntryType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// For a folder with exactly one stage in it, at any depth: that stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<FilePath>,
    /// The files whose pictures show a folder. Absent when that is not known yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<Vec<FilePath>>,
}

/// A picture a page's items show, or that the file has none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PagePicture {
    pub repository: RepositoryId,
    pub path: FilePath,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// One page, with the pictures of what is on it that are ready.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Browse {
    pub items: Vec<BrowseItem>,
    /// How many items there are in all.
    pub total: u64,
    /// Asks for the next page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    pub pictures: Vec<PagePicture>,
    /// The repository's pictures are being drawn ahead of anyone asking, for as long as someone
    /// keeps browsing it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub filling: bool,
}

/// A folder of a repository; none is its top.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FolderRef {
    pub repository: RepositoryId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<FilePath>,
}

/// The arguments of `covers`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = LoreProvider, output = Covers, name = "covers")]
pub struct CoversRequest {
    /// The branch to read; `main` by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchName>,
    /// The folders, at most 64.
    pub folders: Vec<FolderRef>,
}

/// The files that show one folder: at most four, and none when it holds nothing that looks like
/// anything.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cover {
    pub files: Vec<FilePath>,
}

/// What `covers` answers, one for each folder, in their order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Covers {
    pub covers: Vec<Cover>,
}

/// A file of a repository, on a branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileRef {
    pub repository: RepositoryId,
    pub branch: BranchName,
    pub path: FilePath,
}

/// The arguments of `thumbnails`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = LoreProvider, output = FileThumbnails, name = "thumbnails")]
pub struct ThumbnailsRequest {
    /// The files, at most 64.
    pub files: Vec<FileRef>,
    #[serde(default)]
    pub size: ThumbnailSize,
}

/// What is ready of one file: its picture, or none, or `pending` when `thumbnail` has more to say.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadyThumbnail {
    /// The picture's media type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    /// The picture, in base64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pending: bool,
}

/// What `thumbnails` answers, one for each file, in their order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct FileThumbnails {
    pub thumbnails: Vec<ReadyThumbnail>,
}

/// A file's picture, or none.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct FileThumbnail {
    /// The picture's media type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    /// The picture, in base64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}
