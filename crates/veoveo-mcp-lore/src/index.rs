//! What a repository holds at one revision, by one read of its whole tree. A revision fixes
//! everything under it, so what is worked out here stays true for that revision: what shows each
//! folder, and which folders are one object. A folder's page, a search and a cover then need no
//! further read of Lore, however large the repository. Links are listed and not entered, so an
//! index is the same for everyone who may read the repository, and each request still reads the
//! repository's top as its person, so Lore decides whether they read it now.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    contract::{BranchName, RepositoryId},
    hub::{EntryKind, HubError, Reader, TreeEntry},
};

/// The most files on a card.
const MOST_COVER: usize = 4;
/// The documents that look like something; other text shows as small print.
const DOCUMENTS: [&str; 10] = [
    "pdf", "doc", "docx", "odt", "xls", "xlsx", "ods", "ppt", "pptx", "odp",
];
const STAGES: [&str; 4] = ["usd", "usda", "usdc", "usdz"];
const PICTURES: [&str; 29] = [
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "heic", "heif", "avif", "ico",
    "psd", "svg", "tga", "exr", "hdr", "mp4", "m4v", "mov", "mkv", "webm", "avi", "wmv", "flv",
    "mpg", "mpeg", "ogv", "3gp",
];
const MODELS: [&str; 15] = [
    "glb", "gltf", "fbx", "obj", "stl", "ply", "dae", "3ds", "off", "step", "stp", "iges", "igs",
    "brep", "3mf",
];

fn extension(name: &str) -> Option<String> {
    Some(name.rsplit_once('.')?.1.to_ascii_lowercase())
}

pub fn is_stage(name: &str) -> bool {
    extension(name).is_some_and(|extension| STAGES.contains(&extension.as_str()))
}

/// How well a file shows a folder, the lower the better, or nothing when it would not. Text and
/// code would show as pages of small print, and a folder of them is better shown by its icon.
fn rank(name: &str) -> Option<u8> {
    let extension = extension(name)?;
    let is = |names: &[&str]| names.contains(&extension.as_str());
    if is(&STAGES) {
        Some(0)
    } else if is(&PICTURES) {
        Some(1)
    } else if is(&MODELS) {
        Some(2)
    } else if is(&DOCUMENTS) {
        Some(3)
    } else {
        None
    }
}

/// One name in a folder, as a page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub path: String,
    pub kind: EntryKind,
    pub size: Option<u64>,
    /// For a folder with exactly one stage in it, at any depth: that stage. The folder is then one
    /// object, known by the folder's name, whatever the stage's file is called.
    pub stage: Option<String>,
    /// The files whose pictures show a folder.
    pub cover: Vec<String>,
}

#[derive(Debug, Default)]
struct Folder {
    /// Folders first, each kind by name.
    children: Vec<Child>,
    stages: u32,
    only_stage: Option<String>,
    cover: Vec<String>,
}

#[derive(Debug)]
struct Child {
    name: String,
    path: String,
    kind: EntryKind,
    size: Option<u64>,
}

/// A repository at one revision.
#[derive(Debug, Default)]
pub struct Index {
    revision: Option<String>,
    folders: HashMap<String, Folder>,
    addresses: HashMap<String, String>,
}

fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

impl Index {
    pub fn build(entries: Vec<TreeEntry>, revision: Option<String>) -> Self {
        let mut folders: HashMap<String, Folder> = HashMap::new();
        folders.entry(String::new()).or_default();
        let mut addresses = HashMap::new();
        for entry in entries {
            if entry.kind == EntryKind::Directory {
                folders.entry(entry.path.clone()).or_default();
            }
            if let Some(address) = entry.address {
                addresses.insert(entry.path.clone(), address);
            }
            let parent = parent_of(&entry.path).to_owned();
            let name = entry.path.rsplit('/').next().unwrap_or_default().to_owned();
            folders.entry(parent).or_default().children.push(Child {
                name,
                path: entry.path,
                kind: entry.kind,
                size: entry.size,
            });
        }
        // Deepest first, so that each folder finds its folders' answers ready.
        let mut paths: Vec<String> = folders.keys().cloned().collect();
        paths.sort_by_key(|path| {
            std::cmp::Reverse(path.matches('/').count() + usize::from(!path.is_empty()))
        });
        for path in paths {
            let Some(mut folder) = folders.remove(&path) else {
                continue;
            };
            folder.children.sort_by_key(|child| {
                (
                    child.kind != EntryKind::Directory,
                    child.name.to_lowercase(),
                )
            });
            let mut own: Vec<(u8, bool, &str)> = folder
                .children
                .iter()
                .filter(|child| child.kind == EntryKind::File)
                .filter_map(|child| {
                    Some((
                        rank(&child.name)?,
                        !is_stage(&child.name),
                        child.path.as_str(),
                    ))
                })
                .collect();
            own.sort();
            let mut cover: Vec<String> =
                own.iter().map(|(_, _, path)| (*path).to_owned()).collect();
            let mut stages = 0;
            let mut only_stage = None;
            for child in &folder.children {
                match child.kind {
                    EntryKind::File if is_stage(&child.name) => {
                        stages += 1;
                        only_stage = Some(child.path.clone());
                    }
                    EntryKind::Directory => {
                        if let Some(inside) = folders.get(&child.path) {
                            stages += inside.stages;
                            if inside.stages > 0 {
                                only_stage.clone_from(&inside.only_stage);
                            }
                            cover.extend(inside.cover.first().cloned());
                        }
                    }
                    _ => {}
                }
            }
            cover.truncate(MOST_COVER);
            folder.cover = cover;
            folder.stages = stages;
            folder.only_stage = only_stage.filter(|_| stages == 1);
            folders.insert(path, folder);
        }
        Self {
            revision,
            folders,
            addresses,
        }
    }

    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }

    fn item(&self, child: &Child) -> Item {
        let inside = (child.kind == EntryKind::Directory)
            .then(|| self.folders.get(&child.path))
            .flatten();
        Item {
            name: child.name.clone(),
            path: child.path.clone(),
            kind: child.kind,
            size: child.size,
            stage: inside.and_then(|folder| folder.only_stage.clone()),
            cover: inside.map_or_else(Vec::new, |folder| folder.cover.clone()),
        }
    }

    /// What shows a folder.
    pub fn cover_of(&self, folder: &str) -> Vec<String> {
        self.folders
            .get(folder)
            .map_or_else(Vec::new, |folder| folder.cover.clone())
    }

    /// A folder's items from `offset`, at most `limit`, and how many there are in all; none when
    /// the revision has no such folder.
    pub fn page(&self, folder: &str, offset: usize, limit: usize) -> Option<(Vec<Item>, usize)> {
        let children = &self.folders.get(folder)?.children;
        let items = children
            .iter()
            .skip(offset)
            .take(limit)
            .map(|child| self.item(child))
            .collect();
        Some((items, children.len()))
    }

    /// What is under `folder` whose name holds `search`, whatever its case, in the order of a walk.
    /// A walk stops at an object: what is inside one is that object's.
    pub fn search(
        &self,
        folder: &str,
        search: &str,
        offset: usize,
        limit: usize,
    ) -> Option<(Vec<Item>, usize)> {
        let wanted = search.to_lowercase();
        let mut found = 0;
        let mut items = Vec::new();
        let mut stack = vec![self.folders.get(folder)?];
        while let Some(here) = stack.pop() {
            for child in here.children.iter().rev() {
                let inside = (child.kind == EntryKind::Directory)
                    .then(|| self.folders.get(&child.path))
                    .flatten();
                if child.name.to_lowercase().contains(&wanted) {
                    if found >= offset && items.len() < limit {
                        items.push(self.item(child));
                    }
                    found += 1;
                }
                if let Some(inside) = inside.filter(|inside| inside.only_stage.is_none()) {
                    stack.push(inside);
                }
            }
        }
        Some((items, found))
    }

    /// What names what a file's picture shows: its content, and for a stage that is not a package
    /// the revision too, since it may name other files. None for a name the index does not hold.
    pub fn picture_key(&self, path: &str) -> Option<String> {
        let address = self.addresses.get(path)?;
        let name = path.rsplit('/').next().unwrap_or_default();
        Some(
            match (
                &self.revision,
                is_stage(name) && !name.to_lowercase().ends_with(".usdz"),
            ) {
                (Some(revision), true) => format!("{address}-{revision}"),
                _ => address.clone(),
            },
        )
    }
}

/// A branch of a repository.
type Place = (RepositoryId, BranchName);

/// The latest index of each repository's branch.
#[derive(Clone, Default)]
pub struct Indexes {
    held: Arc<Mutex<HashMap<Place, Arc<Index>>>>,
    turns: Arc<Mutex<HashMap<Place, Arc<tokio::sync::Mutex<()>>>>>,
}

impl Indexes {
    /// The latest index, at whatever revision, for what shows on a repository's card: a person who
    /// holds a grant on the repository may see an earlier revision of it.
    pub fn latest(&self, repository: &RepositoryId, branch: &BranchName) -> Option<Arc<Index>> {
        let held = self.held.lock().unwrap_or_else(|held| held.into_inner());
        held.get(&(repository.clone(), branch.clone())).cloned()
    }

    /// The index of the branch's tip, for the reader's person. Their read of the repository's top
    /// is Lore's answer on whether they may read it, and it names the tip; only a new tip costs a
    /// read of the whole tree, and many asking at once read it once.
    pub async fn at_tip(
        &self,
        reader: &Reader,
        repository: &RepositoryId,
        branch: &BranchName,
    ) -> Result<Arc<Index>, HubError> {
        let tip = reader.listing(repository, branch, None).await?.revision;
        let current = |index: &Option<Arc<Index>>| {
            index
                .as_ref()
                .filter(|index| index.revision == tip)
                .cloned()
        };
        if let Some(index) = current(&self.latest(repository, branch)) {
            return Ok(index);
        }
        let key = (repository.clone(), branch.clone());
        let turn = {
            let mut turns = self.turns.lock().unwrap_or_else(|held| held.into_inner());
            turns.entry(key.clone()).or_default().clone()
        };
        let _turn = turn.lock().await;
        if let Some(index) = current(&self.latest(repository, branch)) {
            return Ok(index);
        }
        let started = std::time::Instant::now();
        let tree = reader.tree(repository, branch).await?;
        let read_ms = started.elapsed().as_millis();
        let entries = tree.entries.len();
        let index = Arc::new(Index::build(tree.entries, tree.revision));
        tracing::info!(%repository, entries, read_ms, total_ms = started.elapsed().as_millis(), "read a repository's whole tree");
        let mut held = self.held.lock().unwrap_or_else(|held| held.into_inner());
        held.insert(key, index.clone());
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, kind: EntryKind) -> TreeEntry {
        TreeEntry {
            path: path.to_owned(),
            kind,
            size: Some(3),
            address: (kind == EntryKind::File).then(|| format!("ad-{}", path.len())),
        }
    }

    /// Two objects of a category, each one stage named alike, and a folder of code.
    fn dataset() -> Index {
        use EntryKind::{Directory as D, File as F, Link as L};
        Index::build(
            [
                ("rigid", D),
                ("rigid/kitchen", D),
                ("rigid/kitchen/R001_microwave", D),
                ("rigid/kitchen/R001_microwave/01_compact", D),
                ("rigid/kitchen/R001_microwave/01_compact/meta.json", F),
                ("rigid/kitchen/R001_microwave/01_compact/physx", D),
                (
                    "rigid/kitchen/R001_microwave/01_compact/physx/model.usdc",
                    F,
                ),
                ("rigid/kitchen/R001_microwave/01_compact/physx/textures", D),
                (
                    "rigid/kitchen/R001_microwave/01_compact/physx/textures/microwave_a.png",
                    F,
                ),
                ("rigid/kitchen/R001_microwave/02_steel", D),
                ("rigid/kitchen/R001_microwave/02_steel/physx", D),
                ("rigid/kitchen/R001_microwave/02_steel/physx/model.usdc", F),
                ("src", D),
                ("src/main.rs", F),
                ("elsewhere", L),
                ("README.md", F),
            ]
            .map(|(path, kind)| entry(path, kind))
            .into(),
            Some("abc".to_owned()),
        )
    }

    #[test]
    fn a_folder_with_one_stage_is_one_object_at_any_depth() {
        let index = dataset();
        let (items, total) = index.page("rigid/kitchen/R001_microwave", 0, 10).unwrap();
        assert_eq!(total, 2);
        assert_eq!(items[0].name, "01_compact");
        assert_eq!(
            items[0].stage.as_deref(),
            Some("rigid/kitchen/R001_microwave/01_compact/physx/model.usdc")
        );
        // A folder of two stages is no object, and neither is a folder of none.
        let (top, _) = index.page("", 0, 10).unwrap();
        let rigid = top.iter().find(|item| item.name == "rigid").unwrap();
        assert_eq!(rigid.stage, None);
        assert_eq!(
            top.iter().find(|item| item.name == "src").unwrap().stage,
            None
        );
    }

    #[test]
    fn a_cover_comes_from_any_depth_and_shows_a_stage_before_what_it_uses() {
        let index = dataset();
        assert_eq!(
            index.cover_of("rigid/kitchen/R001_microwave"),
            [
                "rigid/kitchen/R001_microwave/01_compact/physx/model.usdc",
                "rigid/kitchen/R001_microwave/02_steel/physx/model.usdc",
            ]
        );
        assert_eq!(
            index.cover_of("rigid"),
            ["rigid/kitchen/R001_microwave/01_compact/physx/model.usdc"]
        );
        assert!(index.cover_of("src").is_empty());
        assert!(index.cover_of("nowhere").is_empty());
    }

    #[test]
    fn a_page_is_folders_first_and_says_how_many_there_are() {
        let index = dataset();
        let (first, total) = index.page("", 0, 2).unwrap();
        assert_eq!(total, 4);
        let names: Vec<&str> = first.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["rigid", "src"]);
        let (rest, _) = index.page("", 2, 10).unwrap();
        let names: Vec<&str> = rest.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["elsewhere", "README.md"]);
        assert!(index.page("nowhere", 0, 2).is_none());
    }

    #[test]
    fn a_search_finds_names_at_any_depth_and_stops_at_an_object() {
        let index = dataset();
        let (found, total) = index.search("", "MICROWAVE", 0, 10).unwrap();
        // Not the texture inside the object that bears the word.
        assert_eq!(total, 1);
        assert_eq!(found[0].name, "R001_microwave");
        let (found, total) = index.search("", "steel", 0, 10).unwrap();
        assert_eq!(total, 1);
        assert!(found[0].stage.is_some());
        let (found, total) = index.search("", "r", 1, 1).unwrap();
        assert_eq!(found.len(), 1);
        assert!(total > 1);
    }

    #[test]
    fn what_a_picture_shows_follows_content_and_a_stages_revision() {
        let index = dataset();
        let stage = "rigid/kitchen/R001_microwave/02_steel/physx/model.usdc";
        assert!(index.picture_key(stage).unwrap().ends_with("-abc"));
        assert!(!index.picture_key("README.md").unwrap().contains("abc"));
        assert_eq!(index.picture_key("nowhere.png"), None);
    }
}
