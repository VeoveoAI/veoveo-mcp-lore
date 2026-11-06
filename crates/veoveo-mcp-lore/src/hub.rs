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

use crate::contract::{BranchName, FilePath, RepositoryId};

/// How long a listing may take.
const LISTING_TIMEOUT: Duration = Duration::from_secs(30);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    File,
    Directory,
    Link,
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
    async fn get(&self, url: url::Url) -> Result<Answer, HubError> {
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
        let answer = self.get(url).await?;
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
        let answer = self.get(url).await?;
        Ok(Listing {
            revision: answer.revision,
            ..serde_json::from_slice(&answer.body)?
        })
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
