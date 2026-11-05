//! One typed route table binds requests, outputs, discovery and current permissions.
mod declarations;
mod requirements;
mod setup;

use rmcp::{ErrorData as McpError, RoleServer, service::RequestContext};
use std::sync::LazyLock;
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
        BranchName, DEFAULT_BRANCH, EntryType, FileEntry, FileListing, FileThumbnail,
        ListFilesRequest, LorePermission, RepositorySummary, ThumbnailRequest, ThumbnailSize,
    },
    hub::{EntryKind, Hub, HubError, Reader},
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
}

impl LoreMcp {
    pub fn new(hub: Hub) -> std::io::Result<Self> {
        Ok(Self {
            hub,
            app: app_document()?,
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
            size: match request.size {
                ThumbnailSize::Small => thumb::ThumbnailSize::Small,
                ThumbnailSize::Large => thumb::ThumbnailSize::Large,
            },
        };
        let drawn = client
            .call(asked)
            .await
            .map_err(|error| ToolError::Rejected(format!("reading the file: {error}")))?;
        Ok(FileThumbnail {
            media_type: drawn.media_type,
            data: drawn.data,
        })
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
        .app(
            AppBuilder::new(app(), APP_NAME)
                .title(APP_TITLE)
                .description("Browse the Lore repositories you can read, with a picture of each file.")
                .policy(policy())
                .reader(|domain: LoreMcp, _| async move { Ok(domain.app.clone()) }),
        )
        .app_tools(&app(), &[ListFilesRequest::TOOL], &[UiVisibility::Model, UiVisibility::App])
        .app_tools(&app(), &[ThumbnailRequest::TOOL], &[UiVisibility::App])
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
