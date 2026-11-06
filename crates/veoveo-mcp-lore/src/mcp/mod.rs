//! One typed route table binds requests, outputs, discovery and current permissions.
mod declarations;
mod requirements;
mod setup;

use futures::{StreamExt as _, TryStreamExt as _};
use rmcp::{ErrorData as McpError, RoleServer, service::RequestContext};
use std::{
    collections::HashSet,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use veoveo_hub_sdk::{
    apps::{AppHtml, AppUri, UiVisibility},
    calls::ToolCall,
    hosting::{
        AppBuilder, ClientContext, DomainServer, RoutePolicy, Routes, ToolError, hub_identity,
    },
    server_contract::McpServerSetup,
};
use veoveo_mcp_thumb::contract as thumb;

use crate::{
    contract::{
        BranchName, Browse, BrowseItem, BrowseRequest, Cover, Covers, CoversRequest,
        DEFAULT_BRANCH, EntryType, FileEntry, FileListing, FilePath, FileThumbnail, FileThumbnails,
        ListFilesRequest, LorePermission, MOST_AT_ONCE, MOST_ON_A_PAGE, PagePicture,
        ReadyThumbnail, RepositoryId, RepositorySummary, ThumbnailRequest, ThumbnailSize,
        ThumbnailsRequest,
    },
    hub::{EntryKind, Hub, HubError, Reader},
    index::{Index, Indexes, Item, is_stage},
    pictures::{Kept, Pictures},
};

pub use declarations::{REGISTRATION, registration};
pub use requirements::AUTHORIZATION;
pub use setup::LoreContract;
use setup::SERVER_SETUP;

/// The App's address.
pub const APP_URI: &str = "ui://lore-api/repositories";
/// The App's programmatic resource name.
const APP_NAME: &str = "repositories";
/// The App's title.
const APP_TITLE: &str = "Repositories";
/// How many items a page holds unless asked.
const PAGE: u32 = 60;
/// How many reads one call makes at once.
const READS_AT_ONCE: usize = 8;
/// The most base64 one page answers, well inside an MCP response. A page's other pictures are asked
/// for afterwards.
const MOST_PAGE_BYTES: usize = 3 * 1024 * 1024;
/// How many pictures are drawn ahead at once.
const DRAWN_AHEAD: usize = 4;
/// What the App adds to the gallery.
const APP_ADAPTER: &str = include_str!("../../assets/repositories.js");

pub fn force_setup() {
    LazyLock::force(&SERVER_SETUP);
}

/// The App's document.
pub fn app_document() -> std::io::Result<AppHtml> {
    AppHtml::load(&veoveo_hub_ui::gallery_file(APP_TITLE, APP_ADAPTER)?)
}

#[derive(Clone)]
pub struct LoreMcp {
    hub: Hub,
    app: AppHtml,
    indexes: Indexes,
    pictures: Pictures,
    filling: Arc<Mutex<Filling>>,
}

/// Which revisions' pictures are being drawn ahead, and which are done.
#[derive(Default)]
struct Filling {
    running: HashSet<(RepositoryId, String)>,
    finished: HashSet<(RepositoryId, String)>,
}

impl LoreMcp {
    /// Pictures it is handed are kept under `pictures`.
    pub fn new(hub: Hub, pictures: std::path::PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            hub,
            app: app_document()?,
            indexes: Indexes::default(),
            pictures: Pictures::new(pictures)?,
            filling: Arc::default(),
        })
    }

    fn reader(&self, context: &RequestContext<RoleServer>) -> Result<Reader, McpError> {
        let identity = hub_identity(context)?;
        Ok(Reader {
            hub: self.hub.clone(),
            client: context.client()?,
            profile: identity.profile,
        })
    }

    async fn list_files(
        &self,
        request: ListFilesRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<FileListing, ToolError> {
        let reader = self.reader(&context)?;
        let Some(repository) = request.repository else {
            let mut repositories = reader
                .repositories()
                .await
                .map_err(|error| refused("listing your repositories", &error))?;
            repositories.sort_by_key(|repository| repository.name.to_lowercase());
            return Ok(FileListing {
                repositories: repositories
                    .into_iter()
                    .map(|repository| RepositorySummary {
                        id: repository.id,
                        name: repository.name,
                    })
                    .collect(),
                repository: None,
                branch: None,
                path: None,
                entries: vec![],
            });
        };
        let branch = match request.branch {
            Some(branch) => branch,
            None => BranchName::parse(DEFAULT_BRANCH).expect("the default branch's name"),
        };
        let listing = reader
            .listing(&repository, &branch, request.path.as_ref())
            .await
            .map_err(|error| refused("listing the folder", &error))?;
        let mut entries: Vec<FileEntry> = listing
            .entries
            .into_iter()
            .map(|entry| FileEntry {
                name: entry.name,
                entry_type: match entry.kind {
                    EntryKind::File => EntryType::File,
                    EntryKind::Directory => EntryType::Directory,
                    EntryKind::Link => EntryType::Link,
                },
                size: entry.size,
            })
            .collect();
        entries.sort_by_key(|entry| {
            (
                entry.entry_type != EntryType::Directory,
                entry.name.to_lowercase(),
            )
        });
        Ok(FileListing {
            repositories: vec![],
            repository: Some(repository),
            branch: Some(branch),
            path: request.path,
            entries,
        })
    }

    /// One page of what a repository holds, or of the repositories, with the pictures of what is
    /// on it that are ready, so that a screen is one request.
    async fn browse(
        &self,
        request: BrowseRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<Browse, ToolError> {
        let reader = self.reader(&context)?;
        let branch = branch_or_default(request.branch);
        let limit = request.limit.unwrap_or(PAGE).clamp(1, MOST_ON_A_PAGE) as usize;
        let offset = match request.cursor.as_deref() {
            None => 0,
            Some(cursor) => cursor
                .parse()
                .map_err(|_| ToolError::Rejected("not a page of this listing".to_owned()))?,
        };
        let search = request
            .search
            .as_deref()
            .map(str::trim)
            .filter(|search| !search.is_empty());
        let mut filling = false;
        let (items, total, wanted) = match request.repository {
            None => {
                let mut repositories = reader
                    .repositories()
                    .await
                    .map_err(|error| refused("listing your repositories", &error))?;
                repositories.sort_by_key(|repository| repository.name.to_lowercase());
                if let Some(search) = search {
                    let search = search.to_lowercase();
                    repositories
                        .retain(|repository| repository.name.to_lowercase().contains(&search));
                }
                let total = repositories.len();
                let mut wanted = Vec::new();
                let items = repositories
                    .into_iter()
                    .skip(offset)
                    .take(limit)
                    .map(|repository| {
                        let known = self.indexes.latest(&repository.id, &branch);
                        let cover = known.as_ref().map(|index| files(&index.cover_of("")));
                        if let (Some(index), Some(cover)) = (&known, &cover) {
                            for path in cover {
                                wanted.push((repository.id.clone(), index.clone(), path.clone()));
                            }
                        }
                        BrowseItem {
                            name: repository.name,
                            repository: Some(repository.id),
                            path: None,
                            entry_type: EntryType::Directory,
                            size: None,
                            stage: None,
                            cover,
                        }
                    })
                    .collect();
                (items, total, wanted)
            }
            Some(repository) => {
                let index = self
                    .indexes
                    .at_tip(&reader, &repository, &branch)
                    .await
                    .map_err(|error| refused("reading the repository", &error))?;
                filling = self.fill(&reader, &repository, &branch, &index);
                let folder = request.path.as_ref().map_or("", FilePath::as_str);
                let page = match search {
                    Some(search) => index.search(folder, search, offset, limit),
                    None => index.page(folder, offset, limit),
                }
                .ok_or_else(|| ToolError::Rejected("this folder is not there".to_owned()))?;
                let mut wanted = Vec::new();
                let items = page
                    .0
                    .iter()
                    .map(|item| {
                        let browsed = browse_item(item);
                        let shows = match (&browsed.entry_type, &browsed.stage, &browsed.cover) {
                            (EntryType::File, _, _) => browsed.path.clone().into_iter().collect(),
                            (_, Some(stage), _) => vec![stage.clone()],
                            (_, None, Some(cover)) => cover.clone(),
                            _ => Vec::new(),
                        };
                        for path in shows {
                            wanted.push((repository.clone(), index.clone(), path));
                        }
                        browsed
                    })
                    .collect();
                (items, page.1, wanted)
            }
        };
        let end = offset.saturating_add(limit);
        Ok(Browse {
            items,
            total: total as u64,
            next: (end < total).then(|| end.to_string()),
            pictures: self.page_pictures(wanted).await,
            filling,
        })
    }

    /// Draws what a gallery of this revision will ask for, ahead of anyone asking, as the person
    /// browsing, so Lore and each server's rules still decide. It lasts as long as their token: a
    /// later request of anyone browsing starts it again from what is not kept yet. Says whether it
    /// is under way.
    fn fill(
        &self,
        reader: &Reader,
        repository: &RepositoryId,
        branch: &BranchName,
        index: &Arc<Index>,
    ) -> bool {
        let Some(revision) = index.revision() else {
            return false;
        };
        let key = (repository.clone(), revision.to_owned());
        {
            let mut filling = self.filling.lock().unwrap_or_else(|held| held.into_inner());
            if filling.finished.contains(&key) {
                return false;
            }
            if !filling.running.insert(key.clone()) {
                return true;
            }
        }
        let (domain, reader, repository, branch, index) = (
            self.clone(),
            reader.clone(),
            repository.clone(),
            branch.clone(),
            index.clone(),
        );
        tokio::spawn(async move {
            let done = domain
                .draw_ahead(&reader, &repository, &branch, &index)
                .await;
            let mut filling = domain
                .filling
                .lock()
                .unwrap_or_else(|held| held.into_inner());
            filling.running.remove(&key);
            if done {
                filling.finished.insert(key);
            }
        });
        true
    }

    /// Whether everything got an answer. A failed call ends it: the token has likely run out.
    async fn draw_ahead(
        &self,
        reader: &Reader,
        repository: &RepositoryId,
        branch: &BranchName,
        index: &Index,
    ) -> bool {
        let missed = AtomicBool::new(false);
        let missed = &missed;
        let drawn = futures::stream::iter(index.pictured().into_iter().map(Ok::<_, ()>))
            .try_for_each_concurrent(DRAWN_AHEAD, |path| async move {
                let (Some(key), Ok(file)) = (
                    index.picture_key(&path),
                    thumb::FilePath::parse(path.as_str()),
                ) else {
                    return Ok(());
                };
                if self.pictures.get(&key).await.is_some() {
                    return Ok(());
                }
                let (Ok(from), Ok(on)) = (
                    thumb::RepositoryId::parse(repository.as_str()),
                    thumb::BranchName::parse(branch.as_str()),
                ) else {
                    return Err(());
                };
                let answer = reader
                    .client
                    .call(thumb::ThumbnailRequest {
                        source: thumb::FileSource::Lore {
                            repository: from,
                            branch: on,
                            path: file,
                        },
                        size: thumb::ThumbnailSize::Small,
                    })
                    .await
                    .map_err(|error| {
                        tracing::debug!(%error, "a picture was not drawn ahead");
                    })?;
                // An empty answer for a stage may only mean its server is away, so it is left to
                // be asked again, and this revision is not done.
                if answer.data.is_none() && is_stage(&path) {
                    missed.store(true, Ordering::Relaxed);
                    return Ok(());
                }
                self.keep_for(index, &path, &kept_of(answer.media_type, answer.data))
                    .await;
                Ok(())
            })
            .await;
        drawn.is_ok() && !missed.load(Ordering::Relaxed)
    }

    /// What is kept of what a page shows.
    async fn page_pictures(
        &self,
        wanted: Vec<(RepositoryId, Arc<Index>, FilePath)>,
    ) -> Vec<PagePicture> {
        let found: Vec<_> = futures::stream::iter(wanted)
            .map(|(repository, index, path)| async move {
                let key = index.picture_key(path.as_str())?;
                let kept = match self.pictures.get(&key).await {
                    Some(kept) => kept,
                    // The stage's last picture, until drawing ahead asks about this tip.
                    None => {
                        self.pictures
                            .get(&index.content_key(path.as_str())?)
                            .await?
                    }
                };
                Some((repository, path, kept))
            })
            .buffered(READS_AT_ONCE)
            .collect()
            .await;
        let mut left = MOST_PAGE_BYTES;
        found
            .into_iter()
            .flatten()
            .filter_map(|(repository, path, kept)| {
                let (media_type, data) = match kept {
                    Kept::Nothing => (None, None),
                    Kept::Picture { media_type, data } => {
                        left = left.checked_sub(data.len())?;
                        (Some(media_type), Some(data))
                    }
                };
                Some(PagePicture {
                    repository,
                    path,
                    media_type,
                    data,
                })
            })
            .collect()
    }

    /// What shows each folder on its card, which this server decides, so a page never searches.
    async fn covers(
        &self,
        request: CoversRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<Covers, ToolError> {
        if request.folders.len() > MOST_AT_ONCE {
            return Err(ToolError::Rejected(format!(
                "at most {MOST_AT_ONCE} folders at a time"
            )));
        }
        let branch = branch_or_default(request.branch);
        let reader = self.reader(&context)?;
        let reader = &reader;
        let branch = &branch;
        let covers = futures::stream::iter(request.folders)
            .map(|folder| async move {
                // A cover is decoration: what could not be read now shows its icon.
                let Ok(index) = self
                    .indexes
                    .at_tip(reader, &folder.repository, branch)
                    .await
                else {
                    return Cover::default();
                };
                let at = folder.path.as_ref().map_or("", FilePath::as_str);
                Cover {
                    files: files(&index.cover_of(at)),
                }
            })
            .buffered(READS_AT_ONCE)
            .collect()
            .await;
        Ok(Covers { covers })
    }

    /// Keeps a picture under what it shows at this revision, and by content alone for later ones.
    async fn keep_for(&self, index: &Index, path: &str, kept: &Kept) {
        if let Some(key) = index.picture_key(path) {
            self.pictures.keep(&key, kept).await;
        }
        if let (Some(key), Kept::Picture { .. }) = (index.content_key(path), kept) {
            self.pictures.keep(&key, kept).await;
        }
    }

    /// Keeps what the Thumb server answered of a repository's small pictures, under what they show
    /// at the tip.
    async fn keep(
        &self,
        reader: &Reader,
        repository: &RepositoryId,
        branch: &BranchName,
        kept: Vec<(FilePath, Kept)>,
    ) {
        let Ok(index) = self.indexes.at_tip(reader, repository, branch).await else {
            return;
        };
        for (path, kept) in kept {
            self.keep_for(&index, path.as_str(), &kept).await;
        }
    }

    /// What is ready of many files by one request, from the Thumb server, which reads them as the
    /// same person and draws nothing here.
    async fn thumbnails(
        &self,
        request: ThumbnailsRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<FileThumbnails, ToolError> {
        if request.files.len() > MOST_AT_ONCE {
            return Err(ToolError::Rejected(format!(
                "at most {MOST_AT_ONCE} files at a time"
            )));
        }
        // A stage is the USD server's to draw, one at a time, so asking what is ready of it would
        // only cost the calls to say so.
        let (stages, files): (Vec<_>, Vec<_>) = request
            .files
            .iter()
            .enumerate()
            .partition(|(_, file)| is_stage(file.path.file_name()));
        let mut thumbnails = vec![ReadyThumbnail::default(); request.files.len()];
        for (at, _) in &stages {
            thumbnails[*at].pending = true;
        }
        if files.is_empty() {
            return Ok(FileThumbnails { thumbnails });
        }
        let client = context.client().map_err(|error: McpError| {
            ToolError::Rejected(format!("no calls for the caller: {}", error.message))
        })?;
        let sources = files
            .iter()
            .map(|(_, file)| {
                Ok(thumb::FileSource::Lore {
                    repository: thumb::RepositoryId::parse(file.repository.as_str())?,
                    branch: thumb::BranchName::parse(file.branch.as_str())?,
                    path: thumb::FilePath::parse(file.path.as_str())?,
                })
            })
            .collect::<Result<Vec<_>, thumb::ThumbIdError>>()
            .map_err(|error| ToolError::Rejected(error.to_string()))?;
        let ready = client
            .call(thumb::ReadyThumbnailsRequest {
                sources,
                size: thumb_size(request.size),
            })
            .await
            .map_err(|error| ToolError::Rejected(format!("reading the files: {error}")))?;
        let mut by_place: std::collections::BTreeMap<_, Vec<_>> = std::collections::BTreeMap::new();
        for ((at, file), one) in files.iter().zip(ready.thumbnails) {
            if !one.pending && request.size == ThumbnailSize::Small {
                by_place
                    .entry((file.repository.clone(), file.branch.clone()))
                    .or_default()
                    .push((
                        file.path.clone(),
                        kept_of(one.media_type.clone(), one.data.clone()),
                    ));
            }
            thumbnails[*at] = ReadyThumbnail {
                media_type: one.media_type,
                data: one.data,
                pending: one.pending,
            };
        }
        let reader = self.reader(&context)?;
        for ((repository, branch), kept) in by_place {
            self.keep(&reader, &repository, &branch, kept).await;
        }
        Ok(FileThumbnails { thumbnails })
    }

    /// A file's picture, from the Thumb server, which reads the file as the same person. No Thumb
    /// server leaves no picture; a refusal of the file stays a refusal.
    async fn thumbnail(
        &self,
        request: ThumbnailRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<FileThumbnail, ToolError> {
        let client = context.client().map_err(|error: McpError| {
            ToolError::Rejected(format!("no calls for the caller: {}", error.message))
        })?;
        let parse = |what: &str, error: &dyn std::fmt::Display| {
            ToolError::Rejected(format!("{what}: {error}"))
        };
        let asked = thumb::ThumbnailRequest {
            source: thumb::FileSource::Lore {
                repository: thumb::RepositoryId::parse(request.repository.as_str())
                    .map_err(|error| parse("the repository", &error))?,
                branch: thumb::BranchName::parse(request.branch.as_str())
                    .map_err(|error| parse("the branch", &error))?,
                path: thumb::FilePath::parse(request.path.as_str())
                    .map_err(|error| parse("the path", &error))?,
            },
            size: thumb_size(request.size),
        };
        let drawn = client
            .call(asked)
            .await
            .map_err(|error| ToolError::Rejected(format!("reading the file: {error}")))?;
        // An empty answer for a stage may only mean its server is away.
        if request.size == ThumbnailSize::Small
            && (drawn.data.is_some() || !is_stage(request.path.file_name()))
        {
            let kept = kept_of(drawn.media_type.clone(), drawn.data.clone());
            self.keep(
                &self.reader(&context)?,
                &request.repository,
                &request.branch,
                vec![(request.path.clone(), kept)],
            )
            .await;
        }
        Ok(FileThumbnail {
            media_type: drawn.media_type,
            data: drawn.data,
        })
    }
}

fn branch_or_default(branch: Option<BranchName>) -> BranchName {
    branch.unwrap_or_else(|| BranchName::parse(DEFAULT_BRANCH).expect("the default branch's name"))
}

fn files(paths: &[String]) -> Vec<FilePath> {
    paths
        .iter()
        .filter_map(|path| FilePath::parse(path.as_str()).ok())
        .collect()
}

fn browse_item(item: &Item) -> BrowseItem {
    BrowseItem {
        name: item.name.clone(),
        repository: None,
        path: FilePath::parse(item.path.as_str()).ok(),
        entry_type: match item.kind {
            EntryKind::File => EntryType::File,
            EntryKind::Directory => EntryType::Directory,
            EntryKind::Link => EntryType::Link,
        },
        size: item.size,
        stage: item
            .stage
            .as_deref()
            .and_then(|stage| FilePath::parse(stage).ok()),
        cover: (item.kind == EntryKind::Directory).then(|| files(&item.cover)),
    }
}

fn kept_of(media_type: Option<String>, data: Option<String>) -> Kept {
    match (media_type, data) {
        (Some(media_type), Some(data)) => Kept::Picture { media_type, data },
        _ => Kept::Nothing,
    }
}

fn thumb_size(size: ThumbnailSize) -> thumb::ThumbnailSize {
    match size {
        ThumbnailSize::Small => thumb::ThumbnailSize::Small,
        ThumbnailSize::Large => thumb::ThumbnailSize::Large,
    }
}

fn refused(what: &str, error: &HubError) -> ToolError {
    ToolError::Rejected(format!("{what}: {error}"))
}

fn policy() -> RoutePolicy {
    RoutePolicy::permission(LorePermission::FilesRead).expect("the server's permission")
}

fn app() -> AppUri {
    AppUri::parse(APP_URI).expect("the App's address")
}

static ROUTES: LazyLock<Routes<LoreMcp>> = LazyLock::new(|| {
    Routes::<LoreMcp>::builder(policy())
        .uses(thumb::ThumbPermission::ThumbnailsRead.name())
        .calls_hub()
        .tool::<ListFilesRequest, _, _>(
            "List the Lore repositories you can read or, given a `repository`, what a folder of it holds on a branch: `main` and its top by default.",
            policy(),
            |domain, request, context| async move { domain.list_files(request, context).await },
        )
        .tool::<ThumbnailRequest, _, _>(
            "A picture of a file in a Lore repository you can read, in base64, or none.",
            policy(),
            |domain, request, context| async move { domain.thumbnail(request, context).await },
        )
        .tool::<BrowseRequest, _, _>(
            "One page of the repositories you can read or of what a repository holds, or of what is named by `search` at any depth, with the pictures of what is on it that are ready. A folder with exactly one stage in it names that stage.",
            policy(),
            |domain, request, context| async move { domain.browse(request, context).await },
        )
        .tool::<CoversRequest, _, _>(
            "What shows each of many folders on its card: at most four files, or none.",
            policy(),
            |domain, request, context| async move { domain.covers(request, context).await },
        )
        .tool::<ThumbnailsRequest, _, _>(
            "What is ready of many files by one request: each one's picture as base64, or none, or `pending` when `thumbnail` can still make one.",
            policy(),
            |domain, request, context| async move { domain.thumbnails(request, context).await },
        )
        .app(
            AppBuilder::new(app(), APP_NAME)
                .title(APP_TITLE)
                .description("Browse the Lore repositories you can read, with a picture of each file.")
                .policy(policy())
                .reader(|domain: LoreMcp, _| async move { Ok(domain.app.clone()) }),
        )
        .app_tools(&app(), &[ListFilesRequest::TOOL], &[UiVisibility::Model, UiVisibility::App])
        .app_tools(
            &app(),
            &[
                BrowseRequest::TOOL,
                ThumbnailRequest::TOOL,
                CoversRequest::TOOL,
                ThumbnailsRequest::TOOL,
            ],
            &[UiVisibility::App],
        )
        .build()
});

impl DomainServer for LoreMcp {
    type Contract = LoreContract;
    fn setup() -> &'static McpServerSetup<LoreContract> {
        &SERVER_SETUP
    }
    fn routes() -> &'static Routes<Self> {
        &ROUTES
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_setup_and_documents_admit() {
        std::sync::LazyLock::force(&super::SERVER_SETUP);
        assert!(super::setup::SERVER_DOCS.agent_manual().is_some());
    }

    #[test]
    fn the_routes_call_the_hub_and_the_app_document_builds() {
        assert!(super::ROUTES.authorization().calls_hub);
        super::app_document().unwrap();
    }
}
