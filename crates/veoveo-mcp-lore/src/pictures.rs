//! Small pictures this server was handed, kept on disk by what they show, so that a page's one
//! request can answer them with no call to another server. It draws nothing and decides nothing:
//! the Thumb server made each of them, a page names only what its person's read of the repository
//! listed, and all of it can be fetched again.
use std::path::PathBuf;

/// What is kept of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kept {
    Picture {
        media_type: String,
        data: String,
    },
    /// It has no picture.
    Nothing,
}

const NOTHING: &str = "nothing";
const KINDS: [(&str, &str); 3] = [
    ("image/webp", "webp"),
    ("image/jpeg", "jpeg"),
    ("image/png", "png"),
];

#[derive(Clone)]
pub struct Pictures {
    folder: PathBuf,
}

/// A key names a file only if it could be nothing else.
fn safe(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 200
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

impl Pictures {
    pub fn new(folder: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&folder)?;
        Ok(Self { folder })
    }

    pub async fn get(&self, key: &str) -> Option<Kept> {
        if !safe(key) {
            return None;
        }
        for (media_type, kind) in KINDS {
            if let Ok(data) =
                tokio::fs::read_to_string(self.folder.join(format!("{key}.{kind}"))).await
            {
                return Some(Kept::Picture {
                    media_type: media_type.to_owned(),
                    data,
                });
            }
        }
        let nothing = tokio::fs::try_exists(self.folder.join(format!("{key}.{NOTHING}"))).await;
        nothing.unwrap_or(false).then_some(Kept::Nothing)
    }

    /// Keeps a picture, whole or not at all. A failure costs a later fetch.
    pub async fn keep(&self, key: &str, kept: &Kept) {
        if !safe(key) {
            return;
        }
        let (kind, data) = match kept {
            Kept::Nothing => (NOTHING, ""),
            Kept::Picture { media_type, data } => {
                let Some((_, kind)) = KINDS.iter().find(|(known, _)| known == media_type) else {
                    return;
                };
                (*kind, data.as_str())
            }
        };
        let partial = self
            .folder
            .join(format!("{key}.partial-{}", std::process::id()));
        if tokio::fs::write(&partial, data).await.is_ok() {
            let _ = tokio::fs::rename(&partial, self.folder.join(format!("{key}.{kind}"))).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn what_was_kept_comes_back_and_a_strange_key_is_nothing() {
        let folder = tempfile::tempdir().unwrap();
        let pictures = Pictures::new(folder.path().join("kept")).unwrap();
        assert_eq!(pictures.get("ab-1").await, None);
        let picture = Kept::Picture {
            media_type: "image/webp".to_owned(),
            data: "AQID".to_owned(),
        };
        pictures.keep("ab-1", &picture).await;
        assert_eq!(pictures.get("ab-1").await, Some(picture.clone()));
        pictures.keep("ab-2", &Kept::Nothing).await;
        assert_eq!(pictures.get("ab-2").await, Some(Kept::Nothing));
        pictures.keep("../up", &picture).await;
        assert_eq!(pictures.get("../up").await, None);
        assert!(!folder.path().join("up.webp").exists());
        let other = Kept::Picture {
            media_type: "text/html".to_owned(),
            data: "x".to_owned(),
        };
        pictures.keep("ab-3", &other).await;
        assert_eq!(pictures.get("ab-3").await, None);
    }
}
