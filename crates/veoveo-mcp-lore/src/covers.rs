//! What shows a folder on its card: pictures of what it holds, first its own files and then one
//! from each folder inside. Text and code would show as pages of small print, and a folder of them
//! is better shown by its icon, so most folders have no cover and finding that out once is enough.
//!
//! A revision fixes everything under it, so what was found stays true for that revision and is
//! remembered in memory. A person still lists the repository's top first, as themselves, so Lore
//! decides whether they read it now; what is remembered is only names within it.
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

use futures::StreamExt as _;

use crate::{
    contract::{BranchName, Cover, FilePath, FolderRef, RepositoryId},
    hub::{EntryKind, HubError, Listing, Reader},
};

/// The most files on a card.
const MOST_FILES: usize = 4;
/// How many folders inside are looked into, when what a folder holds itself does not fill a card.
const MOST_INSIDE: usize = 6;
/// How many reads one call makes at once.
const READS_AT_ONCE: usize = 8;
/// How many folders are remembered, before starting over.
const MOST_REMEMBERED: usize = 16 * 1024;
/// The documents that look like something; other text shows as small print.
const DOCUMENTS: [&str; 10] = [
    "pdf", "doc", "docx", "odt", "xls", "xlsx", "ods", "ppt", "pptx", "odp",
];
const PICTURES: [&str; 33] = [
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "heic", "heif", "avif", "ico",
    "psd", "svg", "tga", "exr", "hdr", "mp4", "m4v", "mov", "mkv", "webm", "avi", "wmv", "flv",
    "mpg", "mpeg", "ogv", "3gp", "usd", "usda", "usdc", "usdz",
];
const MODELS: [&str; 15] = [
    "glb", "gltf", "fbx", "obj", "stl", "ply", "dae", "3ds", "off", "step", "stp", "iges", "igs",
    "brep", "3mf",
];

/// How well a file shows a folder, the lower the better, or nothing when it would not.
fn rank(name: &str) -> Option<u8> {
    let (_, extension) = name.rsplit_once('.')?;
    let extension = extension.to_ascii_lowercase();
    let is = |names: &[&str]| names.contains(&extension.as_str());
    if is(&PICTURES) {
        Some(0)
    } else if is(&MODELS) {
        Some(1)
    } else if is(&DOCUMENTS) {
        Some(2)
    } else {
        None
    }
}

fn joined(folder: Option<&FilePath>, name: &str) -> Option<FilePath> {
    let path = match folder {
        Some(folder) => format!("{folder}/{name}"),
        None => name.to_owned(),
    };
    FilePath::parse(path).ok()
}

/// A folder's files that would show it, the best first.
fn pictures_in(folder: Option<&FilePath>, listing: &Listing) -> Vec<FilePath> {
    let mut found: Vec<(u8, FilePath)> = listing
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .filter_map(|entry| Some((rank(&entry.name)?, joined(folder, &entry.name)?)))
        .collect();
    found.sort_by_key(|(rank, _)| *rank);
    found.into_iter().map(|(_, path)| path).collect()
}

type Remembered = (RepositoryId, String, Option<FilePath>);

/// What shows each folder, by the revision it was found at.
#[derive(Clone, Default)]
pub struct CoverMemory(Arc<Mutex<HashMap<Remembered, Cover>>>);

impl CoverMemory {
    fn get(&self, key: &Remembered) -> Option<Cover> {
        let held = self.0.lock().unwrap_or_else(|held| held.into_inner());
        held.get(key).cloned()
    }

    fn keep(&self, key: Remembered, cover: Cover) {
        let mut held = self.0.lock().unwrap_or_else(|held| held.into_inner());
        if held.len() >= MOST_REMEMBERED {
            held.clear();
        }
        held.insert(key, cover);
    }
}

/// What shows `folder`, from its own listing and those of the folders inside it.
async fn find(
    reader: &Reader,
    repository: &RepositoryId,
    branch: &BranchName,
    folder: Option<&FilePath>,
    listing: &Listing,
) -> Result<Cover, HubError> {
    let mut files = pictures_in(folder, listing);
    let inside: Vec<FilePath> = listing
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Directory)
        .filter_map(|entry| joined(folder, &entry.name))
        .take(MOST_INSIDE)
        .collect();
    if files.len() < MOST_FILES {
        let mut read = futures::stream::iter(inside)
            .map(|path| async move {
                let listing = reader.listing(repository, branch, Some(&path)).await;
                listing.map(|listing| pictures_in(Some(&path), &listing))
            })
            .buffered(READS_AT_ONCE);
        while files.len() < MOST_FILES {
            let Some(found) = read.next().await else {
                break;
            };
            files.extend(found?.into_iter().next());
        }
    }
    files.truncate(MOST_FILES);
    Ok(Cover { files })
}

/// One repository's folders: its top read as the person, then what is remembered at its revision,
/// and the rest found and remembered.
async fn of_repository(
    reader: &Reader,
    memory: &CoverMemory,
    repository: RepositoryId,
    branch: BranchName,
    folders: Vec<(usize, Option<FilePath>)>,
) -> Vec<(usize, Cover)> {
    let Ok(top) = reader.listing(&repository, &branch, None).await else {
        return folders
            .into_iter()
            .map(|(at, _)| (at, Cover::default()))
            .collect();
    };
    let revision = top.revision.clone();
    let top = &top;
    let repository = &repository;
    let branch = &branch;
    let revision = &revision;
    futures::stream::iter(folders)
        .map(|(at, folder)| async move {
            let key = revision
                .clone()
                .map(|revision| (repository.clone(), revision, folder.clone()));
            if let Some(cover) = key.as_ref().and_then(|key| memory.get(key)) {
                return (at, cover);
            }
            let found = match &folder {
                None => find(reader, repository, branch, None, top).await,
                Some(path) => match reader.listing(repository, branch, Some(path)).await {
                    Ok(listing) => find(reader, repository, branch, Some(path), &listing).await,
                    Err(error) => Err(error),
                },
            };
            match found {
                Ok(cover) => {
                    if let Some(key) = key {
                        memory.keep(key, cover.clone());
                    }
                    (at, cover)
                }
                // A cover is decoration: what could not be read now shows its icon, and is looked
                // for again next time.
                Err(_) => (at, Cover::default()),
            }
        })
        .buffer_unordered(READS_AT_ONCE)
        .collect()
        .await
}

/// What shows each of `folders`, in their order.
pub async fn covers(
    reader: &Reader,
    memory: &CoverMemory,
    branch: &BranchName,
    folders: Vec<FolderRef>,
) -> Vec<Cover> {
    let mut answers = vec![Cover::default(); folders.len()];
    let mut repositories: BTreeMap<RepositoryId, Vec<(usize, Option<FilePath>)>> = BTreeMap::new();
    for (at, folder) in folders.into_iter().enumerate() {
        repositories
            .entry(folder.repository)
            .or_default()
            .push((at, folder.path));
    }
    let found: Vec<Vec<(usize, Cover)>> = futures::stream::iter(repositories)
        .map(|(repository, folders)| {
            of_repository(reader, memory, repository, branch.clone(), folders)
        })
        .buffer_unordered(READS_AT_ONCE)
        .collect()
        .await;
    for (at, cover) in found.into_iter().flatten() {
        answers[at] = cover;
    }
    answers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::Entry;

    fn listing(entries: &[(&str, EntryKind)]) -> Listing {
        Listing {
            revision: None,
            entries: entries
                .iter()
                .map(|(name, kind)| Entry {
                    name: (*name).to_owned(),
                    kind: *kind,
                    size: None,
                })
                .collect(),
        }
    }

    #[test]
    fn only_what_looks_like_something_shows_a_folder_and_the_best_first() {
        use EntryKind::{Directory, File, Link};
        let held = listing(&[
            ("report.pdf", File),
            ("model.glb", File),
            ("notes.md", File),
            ("main.rs", File),
            ("Makefile", File),
            ("Photo.JPG", File),
            ("images", Directory),
            ("other", Link),
        ]);
        let folder = FilePath::parse("a/b").unwrap();
        let found: Vec<String> = pictures_in(Some(&folder), &held)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(found, ["a/b/Photo.JPG", "a/b/model.glb", "a/b/report.pdf"]);
        assert!(pictures_in(None, &listing(&[("main.rs", File), ("README.md", File)])).is_empty());
    }

    #[test]
    fn a_cover_is_remembered_at_its_revision_and_memory_is_bounded() {
        let memory = CoverMemory::default();
        let repository = RepositoryId::parse("r").unwrap();
        let key = |revision: &str| (repository.clone(), revision.to_owned(), None);
        let cover = Cover {
            files: vec![FilePath::parse("a.png").unwrap()],
        };
        memory.keep(key("one"), cover.clone());
        assert_eq!(memory.get(&key("one")), Some(cover));
        assert_eq!(memory.get(&key("two")), None);
        for n in 0..=MOST_REMEMBERED {
            memory.keep(key(&n.to_string()), Cover::default());
        }
        assert_eq!(memory.get(&key("one")), None);
        assert!(memory.0.lock().unwrap().len() <= MOST_REMEMBERED);
    }
}
