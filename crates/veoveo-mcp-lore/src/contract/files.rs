//! The tools: what a repository holds, and a picture of one of its files.
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
