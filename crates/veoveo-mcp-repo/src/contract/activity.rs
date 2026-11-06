//! The tools that say what people did: pushes across the repositories you can read, what each
//! changed, and a file's change. Everyone is a person here: an agent pushes as the person it works
//! for.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{BranchName, FilePath, RepoProvider, RepositoryId, Revision};

/// The most pushes one `summaries` names: about a screen.
pub const MOST_SUMMARIES: usize = 64;

/// The arguments of `activity`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = RepoProvider, output = Activity, name = "activity")]
pub struct ActivityRequest {
    /// Only this repository. Without one, every repository you can read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryId>,
    /// Only this branch of the repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchName>,
    /// The `next` of the page before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// How many pushes, at most 50.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// A page of pushes, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Activity {
    pub pushes: Vec<Push>,
    /// Pass as `cursor` for the next page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// Each branch of the repository asked for, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<BranchRow>,
}

/// One revision someone pushed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Push {
    pub repository: RepositoryId,
    pub repository_name: String,
    pub branch: BranchName,
    pub revision: Revision,
    pub number: u64,
    pub message: String,
    /// When, in milliseconds since the epoch.
    pub at: u64,
    pub by: PushedBy,
    /// The revision before it, which may be on the branch it forked from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Revision>,
    /// Where its branch started, for what the whole branch changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_at: Option<Revision>,
    /// What it merged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merged: Option<MergedFrom>,
    /// What the factory is doing with it now. The factory keeps no history of builds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<Running>,
}

/// Who pushed it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PushedBy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Whether it is you.
    pub yours: bool,
}

/// What a merge brought in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MergedFrom {
    pub revision: Revision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// A revision the factory is serving, or failed on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Running {
    /// The server that serves it, when it has a name yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// The link it serves through, when it is a linked repository's revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// The branch of the repository it is linked into.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<String>,
    pub serving: bool,
    /// The step that failed on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_message: Option<String>,
    /// What still serves while this one failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub still_serving: Option<Revision>,
    /// The instance's repository, for the factory's page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_repository: Option<RepositoryId>,
    /// The instance's branch, for the factory's page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_branch: Option<String>,
}

/// One branch of a repository, with its latest push and what the factory does with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BranchRow {
    pub name: BranchName,
    /// None while the branch has no revision of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest: Option<Push>,
    /// The factory's word for it, as its page says: `served`, `no_run`, `name_refused`,
    /// `prepare_failed`, `links_failed`, `pending` or `unreadable`. None for a repository nobody
    /// watches, which the factory does not build.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// The arguments of `changes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = RepoProvider, output = Changes, name = "changes")]
pub struct ChangesRequest {
    pub repository: RepositoryId,
    pub revision: Revision,
    /// What to compare with; the revision before it by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub against: Option<Revision>,
}

/// What a revision changed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Changes {
    /// What it was compared with; none for the first revision of a repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub against: Option<Revision>,
    pub changes: Vec<Change>,
    /// Whether the list stopped short.
    #[serde(default)]
    pub truncated: bool,
    /// Whether Lore would not compare them, because the change is too large.
    #[serde(default)]
    pub too_large: bool,
}

/// What happened at one path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Change {
    pub path: String,
    /// Where it moved or was copied from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_path: Option<String>,
    pub action: ChangeAction,
    #[serde(rename = "type")]
    pub kind: ChangeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<LinkChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ChangeAction {
    Added,
    Removed,
    Changed,
    Moved,
    Copied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    File,
    Link,
}

/// A link's move. Someone without a grant on the linked repository sees only `reached: false`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinkChange {
    pub reached: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Revision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Revision>,
    #[serde(default)]
    pub tracking: bool,
}

/// The arguments of `link_pushes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = RepoProvider, output = LinkPushes, name = "link_pushes")]
pub struct LinkPushesRequest {
    /// The linked repository.
    pub repository: RepositoryId,
    /// The revision the link moved to.
    pub to: Revision,
    /// The revision it moved from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Revision>,
}

/// The pushes a link's move brought in, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct LinkPushes {
    pub pushes: Vec<Push>,
    /// Whether there are more than these.
    #[serde(default)]
    pub truncated: bool,
}

/// One push by its repository and revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PushRef {
    pub repository: RepositoryId,
    pub revision: Revision,
}

/// The arguments of `summaries`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = RepoProvider, output = Summaries, name = "summaries")]
pub struct SummariesRequest {
    /// At most 64.
    pub pushes: Vec<PushRef>,
}

/// How much each push changed, in the order asked.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Summaries {
    pub summaries: Vec<Summary>,
}

/// How much one push changed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    pub files: u32,
    pub links: u32,
    /// Up to three changed files that have a picture.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pictures: Vec<FilePath>,
    /// Whether the counts stopped short.
    #[serde(default)]
    pub more: bool,
    /// Whether it could not be read.
    #[serde(default)]
    pub unknown: bool,
}

/// The arguments of `file_diff`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = RepoProvider, output = FileDiff, name = "file_diff")]
pub struct FileDiffRequest {
    pub repository: RepositoryId,
    pub path: FilePath,
    /// The file's path before, when it moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_path: Option<FilePath>,
    /// The revision before; none for a file that was added.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Revision>,
    /// The revision after; none for a file that was removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Revision>,
}

/// A text file's change as a unified diff.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct FileDiff {
    /// The diff; none when it is `binary` or `tooLarge`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<String>,
    pub added: u32,
    pub removed: u32,
    #[serde(default)]
    pub binary: bool,
    #[serde(default)]
    pub too_large: bool,
}
