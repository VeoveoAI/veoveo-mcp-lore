//! The Hub's own routes, called for a tool's caller with the bearer the SDK exchanges for their
//! profile. The Hub decides each call, so Lore's rules decide each read; its refusals reach the
//! caller with their status and RFC 9457 problem detail.
use std::{fmt, time::Duration};

use reqwest::StatusCode;
use serde::Deserialize;
use veoveo_hub_sdk::{
    HubProfileId,
    client::{CallError, HubClient},
};

use crate::contract::{BranchName, Change, FilePath, RepositoryId, Revision};

/// How long a listing may take.
const LISTING_TIMEOUT: Duration = Duration::from_secs(30);
/// How long a whole tree may take.
const TREE_TIMEOUT: Duration = Duration::from_secs(180);
/// How long a comparison may take.
const CHANGES_TIMEOUT: Duration = Duration::from_secs(150);
/// The header that names the revision a read was made at.
const REVISION: &str = "lore-revision";

/// The Hub at the origin of its issuer.
#[derive(Clone)]
pub struct Hub {
    base: url::Url,
    http: reqwest::Client,
}

/// Why a call to the Hub produced no answer.
#[derive(Debug, thiserror::Error)]
pub enum HubError {
    /// The SDK obtained no bearer for the caller.
    #[error("no token for the caller: {0}")]
    Bearer(#[from] CallError),
    #[error("the Hub could not be reached: {0}")]
    Transport(reqwest::Error),
    /// The Hub answered with an error status.
    #[error("the Hub answered {status}{problem}")]
    Answered {
        status: StatusCode,
        problem: Problem,
    },
    #[error("the Hub's answer is not what the server reads: {0}")]
    Decode(#[from] serde_json::Error),
    /// The Hub answered for a revision without naming it.
    #[error("the Hub's answer does not name the revision")]
    NoSuchRevision,
}

impl From<reqwest::Error> for HubError {
    fn from(error: reqwest::Error) -> Self {
        // A URL names a repository and a path, and holds no credential, but it has no place in
        // a caller's error text either.
        Self::Transport(error.without_url())
    }
}

/// The RFC 9457 problem detail the Hub answers an error with.
#[derive(Debug, Default, Deserialize)]
pub struct Problem {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for part in [&self.title, &self.detail].into_iter().flatten() {
            write!(f, ": {part}")?;
        }
        Ok(())
    }
}

/// A repository the caller holds a grant on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Repository {
    pub id: RepositoryId,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct Repositories {
    repositories: Vec<Repository>,
}

/// What a folder holds, and the revision of the branch it was read at, when it has one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Listing {
    pub entries: Vec<Entry>,
    #[serde(skip)]
    pub revision: Option<String>,
}

/// One name in a folder.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Entry {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: EntryKind,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    File,
    Directory,
    Link,
}

/// Everything on a branch's tip.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
    #[serde(skip)]
    pub revision: Option<String>,
}

/// One name in a tree, by its whole path.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TreeEntry {
    pub path: String,
    #[serde(rename = "type")]
    pub kind: EntryKind,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub address: Option<String>,
}

/// A live branch of a repository.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub latest: Option<Revision>,
    #[serde(default)]
    pub forked_at: Option<Revision>,
}

#[derive(Debug, Deserialize)]
struct Branches {
    branches: Vec<Branch>,
}

/// A page of a branch's revisions, newest first, and where the next page starts.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HistoryPage {
    pub revisions: Vec<RevisionRecord>,
    #[serde(default)]
    pub next: Option<Revision>,
}

/// What Lore says of a revision.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionRecord {
    pub revision: Revision,
    #[serde(default)]
    pub branch_id: Option<String>,
    pub number: u64,
    pub message: String,
    pub at: u64,
    pub by: Committer,
    #[serde(default)]
    pub parent: Option<Revision>,
    #[serde(default)]
    pub merged: Option<Merged>,
}

/// Who committed a revision.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Committer {
    #[serde(default)]
    pub principal: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

/// What a revision merged.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Merged {
    pub revision: Revision,
    #[serde(default)]
    pub branch: Option<String>,
}

/// What a revision changed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ChangeList {
    #[serde(default)]
    pub against: Option<Revision>,
    pub changes: Vec<Change>,
    #[serde(default)]
    pub truncated: bool,
}

/// What of the factory this server shows, read as the caller. The Hub's answer says more.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Factory {
    pub caller: String,
    pub repositories: Vec<FactoryRepository>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FactoryRepository {
    pub id: String,
    pub instances: Vec<FactoryInstance>,
    pub branches: Vec<FactoryBranch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FactoryBranch {
    pub name: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FactoryInstance {
    pub branch: String,
    #[serde(default)]
    pub detail: Option<FactoryInstanceDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FactoryInstanceDetail {
    #[serde(default)]
    pub server: Option<String>,
    pub status: InstanceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStatus {
    #[serde(default)]
    pub applied_revision: Option<String>,
    #[serde(default)]
    pub serving_revision: Option<String>,
    #[serde(default)]
    pub failed: Option<Failure>,
    #[serde(default)]
    pub runner: Option<RunnerStatus>,
    #[serde(default)]
    pub runner_failure: Option<RunnerFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Failure {
    pub stage: String,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerStatus {
    pub link: String,
    pub linked_repository: String,
    pub tracked_branch: TrackedBranch,
    #[serde(default)]
    pub linked_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TrackedBranch {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RunnerFailure {
    pub revision: String,
    pub message: String,
}

/// A file at a revision, up to what a diff reads of one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAt {
    Bytes(Vec<u8>),
    TooLarge,
    Absent,
}

/// A successful answer of the Hub.
struct Answer {
    body: Vec<u8>,
    revision: Option<String>,
}

/// The Hub, for one caller at one profile.
#[derive(Clone)]
pub struct Reader {
    pub hub: Hub,
    pub client: HubClient,
    pub profile: HubProfileId,
}

impl Hub {
    pub fn new(issuer: &url::Url, http: reqwest::Client) -> anyhow::Result<Self> {
        anyhow::ensure!(
            issuer.has_host() && !issuer.cannot_be_a_base(),
            "the Hub's issuer is no HTTP origin"
        );
        Ok(Self {
            base: issuer.clone(),
            http,
        })
    }

    /// A route of `segments`, each percent-encoded as one path segment.
    fn route<'a>(&self, segments: impl IntoIterator<Item = &'a str>) -> url::Url {
        let mut url = self.base.clone();
        url.set_query(None);
        url.set_fragment(None);
        url.path_segments_mut()
            .expect("an HTTP origin has a path")
            .clear()
            .extend(segments);
        url
    }
}

impl Reader {
    async fn get(&self, url: url::Url, timeout: Duration) -> Result<Answer, HubError> {
        let bearer = self.client.hub_bearer().await?;
        // A read changes nothing, and a connection can drop between requests, so one that got no
        // answer at all is sent again. One that ran out of time has had its time.
        let send = || {
            self.hub
                .http
                .get(url.clone())
                .bearer_auth(bearer.expose())
                .timeout(timeout)
                .send()
        };
        let mut response = send().await;
        for wait in [200, 1000] {
            match &response {
                Err(error) if !error.is_timeout() => {
                    tokio::time::sleep(Duration::from_millis(wait)).await;
                    response = send().await;
                }
                _ => break,
            }
        }
        let response = response?;
        let status = response.status();
        let revision = response
            .headers()
            .get(REVISION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = response.bytes().await?;
        if status.is_success() {
            return Ok(Answer {
                body: body.to_vec(),
                revision,
            });
        }
        Err(HubError::Answered {
            status,
            problem: serde_json::from_slice(&body).unwrap_or_default(),
        })
    }

    /// `GET /lore/{profile}/repositories`: the repositories the caller holds a grant on.
    pub async fn repositories(&self) -> Result<Vec<Repository>, HubError> {
        let url = self
            .hub
            .route(["lore", self.profile.as_str(), "repositories"]);
        let answer = self.get(url, LISTING_TIMEOUT).await?;
        Ok(serde_json::from_slice::<Repositories>(&answer.body)?.repositories)
    }

    /// A folder's listing; the Hub takes a trailing `/` for a folder.
    pub async fn listing(
        &self,
        repository: &RepositoryId,
        branch: &BranchName,
        folder: Option<&FilePath>,
    ) -> Result<Listing, HubError> {
        let url = self.hub.route(
            [
                "lore",
                self.profile.as_str(),
                "repositories",
                repository.as_str(),
                "files",
                branch.as_str(),
            ]
            .into_iter()
            .chain(folder.into_iter().flat_map(FilePath::segments))
            .chain([""]),
        );
        let answer = self.get(url, LISTING_TIMEOUT).await?;
        Ok(Listing {
            revision: answer.revision,
            ..serde_json::from_slice(&answer.body)?
        })
    }
}

impl Reader {
    /// `GET /lore/{profile}/repositories/{id}/tree/{branch}`: everything on the branch's tip, by
    /// one read.
    pub async fn tree(
        &self,
        repository: &RepositoryId,
        branch: &BranchName,
    ) -> Result<Tree, HubError> {
        let url = self.hub.route([
            "lore",
            self.profile.as_str(),
            "repositories",
            repository.as_str(),
            "tree",
            branch.as_str(),
        ]);
        let answer = self.get(url, TREE_TIMEOUT).await?;
        Ok(Tree {
            revision: answer.revision,
            ..serde_json::from_slice(&answer.body)?
        })
    }
}

impl Reader {
    /// `GET /lore/{profile}/repositories/{id}/branches`: its live branches, by name.
    pub async fn branches(&self, repository: &RepositoryId) -> Result<Vec<Branch>, HubError> {
        let url = self.hub.route([
            "lore",
            self.profile.as_str(),
            "repositories",
            repository.as_str(),
            "branches",
        ]);
        let answer = self.get(url, LISTING_TIMEOUT).await?;
        Ok(serde_json::from_slice::<Branches>(&answer.body)?.branches)
    }

    /// `GET /lore/{profile}/repositories/{id}/history/{branch}`: up to `count` revisions, newest
    /// first, from `from` or its tip.
    pub async fn history(
        &self,
        repository: &RepositoryId,
        branch: &str,
        from: Option<&Revision>,
        count: u32,
    ) -> Result<HistoryPage, HubError> {
        let mut url = self.hub.route([
            "lore",
            self.profile.as_str(),
            "repositories",
            repository.as_str(),
            "history",
            branch,
        ]);
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("count", &count.to_string());
            if let Some(from) = from {
                query.append_pair("from", from.as_str());
            }
        }
        let answer = self.get(url, LISTING_TIMEOUT).await?;
        Ok(serde_json::from_slice(&answer.body)?)
    }

    /// `GET /lore/{profile}/repositories/{id}/changes/{revision}`: what it changed, compared with
    /// `against` or the revision before it.
    pub async fn changes(
        &self,
        repository: &RepositoryId,
        revision: &Revision,
        against: Option<&Revision>,
    ) -> Result<ChangeList, HubError> {
        let mut url = self.hub.route([
            "lore",
            self.profile.as_str(),
            "repositories",
            repository.as_str(),
            "changes",
            revision.as_str(),
        ]);
        if let Some(against) = against {
            url.query_pairs_mut()
                .append_pair("against", against.as_str());
        }
        let answer = self.get(url, CHANGES_TIMEOUT).await?;
        Ok(serde_json::from_slice(&answer.body)?)
    }

    /// `GET /factory/{profile}/instances`: the factory as the caller may see it.
    pub async fn factory(&self) -> Result<Factory, HubError> {
        let url = self
            .hub
            .route(["factory", self.profile.as_str(), "instances"]);
        let answer = self.get(url, CHANGES_TIMEOUT).await?;
        Ok(serde_json::from_slice(&answer.body)?)
    }

    /// A file at one revision, or that it is larger than `most` bytes, or that it is not there.
    pub async fn file_at(
        &self,
        repository: &RepositoryId,
        revision: &Revision,
        path: &FilePath,
        most: u64,
    ) -> Result<FileAt, HubError> {
        let branch = revision.as_branch();
        let url = self.hub.route(
            [
                "lore",
                self.profile.as_str(),
                "repositories",
                repository.as_str(),
                "files",
                branch.as_str(),
            ]
            .into_iter()
            .chain(path.segments()),
        );
        let bearer = self.client.hub_bearer().await?;
        let response = self
            .hub
            .http
            .get(url)
            .bearer_auth(bearer.expose())
            .timeout(LISTING_TIMEOUT)
            .send()
            .await?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(FileAt::Absent);
        }
        if !status.is_success() {
            let body = response.bytes().await?;
            return Err(HubError::Answered {
                status,
                problem: serde_json::from_slice(&body).unwrap_or_default(),
            });
        }
        if response
            .content_length()
            .is_some_and(|length| length > most)
        {
            return Ok(FileAt::TooLarge);
        }
        let body = response.bytes().await?;
        if body.len() as u64 > most {
            return Ok(FileAt::TooLarge);
        }
        Ok(FileAt::Bytes(body.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_listing_decodes_with_what_it_does_not_use() {
        let listing: Listing = serde_json::from_str(
            r#"{"revision":"r","entries":[{"name":"a.png","type":"file","size":3,"address":"x"},{"name":"d","type":"directory"}]}"#,
        )
        .unwrap();
        assert_eq!(listing.entries[0].kind, EntryKind::File);
        assert_eq!(listing.entries[0].size, Some(3));
        assert_eq!(listing.entries[1].kind, EntryKind::Directory);
    }
}
