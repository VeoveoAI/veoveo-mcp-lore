//! What people did in the repositories a person can read, newest first. Lore's records come through
//! the Hub as that person. A revision never changes, so what Lore says of one is held in memory by
//! revision, but a page names only what that person's own reads of their repositories and branches
//! listed now, so what is kept reaches no one else.
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use futures::{StreamExt as _, stream};

use crate::{
    contract::{
        BranchName, BranchRow, MergedFrom, Push, PushedBy, RepositoryId, Revision, Running, Summary,
    },
    hub::{Branch, Factory, HubError, Reader, RevisionRecord},
};

/// How many reads one call makes at once.
pub const READS_AT_ONCE: usize = 8;
/// How many revisions one read past a branch's tip asks for.
const WALK: u32 = 10;
/// The most pushes one list reaches back.
const MOST_DEEP: usize = 1000;
/// How much is held before starting over, each being small and read again when asked.
const MOST_KEPT: usize = 50_000;

/// A revision Lore described, and the one before it on its branch.
struct Kept {
    record: RevisionRecord,
    next: Option<Revision>,
}

type Key = (RepositoryId, Revision);

/// What is known of revisions and of what they changed.
#[derive(Clone, Default)]
pub struct Activities {
    kept: Arc<Mutex<HashMap<Key, Arc<Kept>>>>,
    summaries: Arc<Mutex<HashMap<Key, Summary>>>,
}

/// A repository's name, and one branch of it to read back along.
#[derive(Clone)]
pub struct Place {
    pub repository: RepositoryId,
    pub name: String,
    pub branch: Branch,
}

impl Activities {
    fn known(&self, key: &Key) -> Option<Arc<Kept>> {
        self.kept
            .lock()
            .unwrap_or_else(|held| held.into_inner())
            .get(key)
            .cloned()
    }

    fn remember(
        &self,
        repository: &RepositoryId,
        records: Vec<RevisionRecord>,
        next: Option<Revision>,
    ) {
        let mut kept = self.kept.lock().unwrap_or_else(|held| held.into_inner());
        if kept.len() >= MOST_KEPT {
            kept.clear();
        }
        let nexts = records
            .iter()
            .skip(1)
            .map(|record| Some(record.revision.clone()))
            .chain([next]);
        let answered = records.iter().cloned().zip(nexts).collect::<Vec<_>>();
        for (record, next) in answered {
            kept.insert(
                (repository.clone(), record.revision.clone()),
                Arc::new(Kept { record, next }),
            );
        }
    }

    /// What Lore says of `revision`, reading `count` revisions back from it when it is not known.
    async fn read(
        &self,
        reader: &Reader,
        repository: &RepositoryId,
        branch: &str,
        revision: &Revision,
        count: u32,
    ) -> Result<Arc<Kept>, HubError> {
        let key = (repository.clone(), revision.clone());
        if let Some(found) = self.known(&key) {
            return Ok(found);
        }
        let page = reader
            .history(repository, branch, Some(revision), count)
            .await?;
        self.remember(repository, page.revisions, page.next);
        self.known(&key).ok_or(HubError::NoSuchRevision)
    }

    /// Up to `count` pushes across `places`, newest first. A branch that has no revision of its own
    /// yet has none here, since the branch it forked from says them. A branch Lore does not answer
    /// for leaves the others to be read.
    pub async fn newest(
        &self,
        reader: &Reader,
        places: Vec<Place>,
        count: usize,
    ) -> Vec<(Place, RevisionRecord)> {
        let count = count.min(MOST_DEEP);
        let mut heads = stream::iter(places)
            .map(|place| async move {
                let tip = place.branch.latest.clone()?;
                let kept = self
                    .read(reader, &place.repository, &place.branch.name, &tip, 1)
                    .await
                    .inspect_err(|error| tracing::warn!(%error, "a branch's tip could not be read"))
                    .ok()?;
                (kept.record.branch_id.as_deref() == Some(place.branch.id.as_str()))
                    .then_some((place, kept))
            })
            .buffer_unordered(READS_AT_ONCE)
            .filter_map(std::future::ready)
            .collect::<Vec<_>>()
            .await;
        let mut pushes = Vec::new();
        let mut shown = HashSet::new();
        while pushes.len() < count {
            let Some(newest) = (0..heads.len()).max_by_key(|at| {
                let record = &heads[*at].1.record;
                (record.at, record.number)
            }) else {
                break;
            };
            let (place, kept) = heads.swap_remove(newest);
            if shown.insert((place.repository.clone(), kept.record.revision.clone())) {
                pushes.push((place.clone(), kept.record.clone()));
            }
            // A full page reads no further.
            let Some(next) = kept.next.clone().filter(|_| pushes.len() < count) else {
                continue;
            };
            match self
                .read(reader, &place.repository, &place.branch.name, &next, WALK)
                .await
            {
                Ok(next) => heads.push((place, next)),
                Err(error) => tracing::warn!(%error, "a branch's history could not be read"),
            }
        }
        pushes
    }

    /// The pushes from `to` back to `from`, at most `count`, and whether there are more.
    pub async fn between(
        &self,
        reader: &Reader,
        repository: &RepositoryId,
        to: &Revision,
        from: Option<&Revision>,
        count: usize,
    ) -> Result<(Vec<RevisionRecord>, bool), HubError> {
        let mut records = Vec::new();
        let mut at = Some(to.clone());
        while let Some(revision) = at {
            if from == Some(&revision) {
                return Ok((records, false));
            }
            if records.len() == count {
                return Ok((records, true));
            }
            // The route takes any branch's name when it is told where to start.
            let kept = self
                .read(reader, repository, "main", &revision, WALK)
                .await?;
            records.push(kept.record.clone());
            at = kept.next.clone();
        }
        Ok((records, false))
    }

    pub fn summary(&self, repository: &RepositoryId, revision: &Revision) -> Option<Summary> {
        self.summaries
            .lock()
            .unwrap_or_else(|held| held.into_inner())
            .get(&(repository.clone(), revision.clone()))
            .cloned()
    }

    pub fn keep_summary(&self, repository: &RepositoryId, revision: &Revision, summary: &Summary) {
        let mut summaries = self
            .summaries
            .lock()
            .unwrap_or_else(|held| held.into_inner());
        if summaries.len() >= MOST_KEPT {
            summaries.clear();
        }
        summaries.insert((repository.clone(), revision.clone()), summary.clone());
    }
}

/// `record` as a push, as `factory` knows it now, for its caller.
pub fn push_of(place: &Place, record: &RevisionRecord, factory: Option<&Factory>) -> Option<Push> {
    Some(Push {
        repository: place.repository.clone(),
        repository_name: place.name.clone(),
        branch: BranchName::parse(place.branch.name.clone()).ok()?,
        revision: record.revision.clone(),
        number: record.number,
        message: record.message.clone(),
        at: record.at,
        by: PushedBy {
            name: record.by.name.clone(),
            email: record.by.email.clone(),
            yours: factory.is_some_and(|factory| {
                record.by.principal.as_deref() == Some(factory.caller.as_str())
            }),
        },
        parent: record.parent.clone(),
        forked_at: place.branch.forked_at.clone(),
        merged: record.merged.as_ref().map(|merged| MergedFrom {
            revision: merged.revision.clone(),
            branch: merged.branch.clone(),
        }),
        running: factory.and_then(|factory| {
            running(
                factory,
                &place.repository,
                &place.branch.name,
                &record.revision,
            )
        }),
    })
}

/// What the factory is doing with this revision now.
pub fn running(
    factory: &Factory,
    repository: &RepositoryId,
    branch: &str,
    revision: &Revision,
) -> Option<Running> {
    let revision = revision.as_str();
    for held in &factory.repositories {
        for instance in &held.instances {
            let Some(detail) = &instance.detail else {
                continue;
            };
            let status = &detail.status;
            let serving = status.serving_revision.as_deref();
            let failure = status.failed.as_ref();
            match &status.runner {
                None if held.id == repository.as_str() && instance.branch == branch => {
                    if serving == Some(revision) {
                        return Some(Running {
                            server: detail.server.clone(),
                            serving: true,
                            ..on_instance(held.id.as_str(), &instance.branch)
                        });
                    }
                    if status.applied_revision.as_deref() == Some(revision)
                        && let Some(failure) = failure
                    {
                        return Some(Running {
                            server: detail.server.clone(),
                            failed_stage: Some(failure.stage.clone()),
                            failed_message: failure.message.clone(),
                            still_serving: serving.and_then(|tip| Revision::parse(tip).ok()),
                            ..on_instance(held.id.as_str(), &instance.branch)
                        });
                    }
                }
                Some(runner)
                    if runner.linked_repository == repository.as_str()
                        && runner.tracked_branch.name == branch =>
                {
                    let linked = runner.linked_revision.as_deref();
                    if let Some(refused) = &status.runner_failure
                        && refused.revision == revision
                    {
                        return Some(Running {
                            server: detail.server.clone(),
                            link: Some(runner.link.clone()),
                            on: Some(instance.branch.clone()),
                            failed_stage: Some("link".to_owned()),
                            failed_message: Some(refused.message.clone()),
                            still_serving: linked.and_then(|tip| Revision::parse(tip).ok()),
                            ..on_instance(held.id.as_str(), &instance.branch)
                        });
                    }
                    if linked == Some(revision) && serving.is_some() {
                        return Some(Running {
                            server: detail.server.clone(),
                            link: Some(runner.link.clone()),
                            on: Some(instance.branch.clone()),
                            serving: true,
                            ..on_instance(held.id.as_str(), &instance.branch)
                        });
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn on_instance(repository: &str, branch: &str) -> Running {
    Running {
        server: None,
        link: None,
        on: None,
        serving: false,
        failed_stage: None,
        failed_message: None,
        still_serving: None,
        instance_repository: RepositoryId::parse(repository).ok(),
        instance_branch: Some(branch.to_owned()),
    }
}

/// The factory's word for each branch of a repository it reads, by name.
pub fn branch_rows(
    repository: &RepositoryId,
    branches: &[Branch],
    pushes: &HashMap<String, Push>,
    factory: Option<&Factory>,
) -> Vec<BranchRow> {
    let held = factory.and_then(|factory| {
        factory
            .repositories
            .iter()
            .find(|held| held.id == repository.as_str())
    });
    branches
        .iter()
        .filter_map(|branch| {
            Some(BranchRow {
                name: BranchName::parse(branch.name.clone()).ok()?,
                latest: pushes.get(&branch.name).cloned(),
                state: held.and_then(|held| {
                    held.branches
                        .iter()
                        .find(|listed| listed.name == branch.name)
                        .map(|listed| listed.state.clone())
                }),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::{
        Committer, FactoryInstance, FactoryInstanceDetail, FactoryRepository, Failure,
        InstanceStatus, RunnerStatus, TrackedBranch,
    };

    const REPOSITORY: &str = "0123456789abcdef0123456789abcdef";
    const OTHER: &str = "fedcba9876543210fedcba9876543210";

    fn revision(byte: char) -> Revision {
        Revision::parse(byte.to_string().repeat(64)).unwrap()
    }

    fn repository() -> RepositoryId {
        RepositoryId::parse(REPOSITORY).unwrap()
    }

    fn factory(instances: Vec<FactoryInstance>) -> Factory {
        Factory {
            caller: "me".to_owned(),
            repositories: vec![FactoryRepository {
                id: REPOSITORY.to_owned(),
                instances,
                branches: Vec::new(),
            }],
        }
    }

    fn instance(status: InstanceStatus) -> FactoryInstance {
        FactoryInstance {
            branch: "main".to_owned(),
            detail: Some(FactoryInstanceDetail {
                server: Some("n8n".to_owned()),
                status,
            }),
        }
    }

    #[test]
    fn a_served_revision_says_who_serves_it() {
        let factory = factory(vec![instance(InstanceStatus {
            serving_revision: Some(revision('a').to_string()),
            ..Default::default()
        })]);
        let found = running(&factory, &repository(), "main", &revision('a')).unwrap();
        assert!(found.serving);
        assert_eq!(found.server.as_deref(), Some("n8n"));
        assert_eq!(
            running(&factory, &repository(), "main", &revision('b')),
            None
        );
        assert_eq!(
            running(&factory, &repository(), "probe", &revision('a')),
            None
        );
    }

    #[test]
    fn a_failed_revision_says_why_and_what_still_serves() {
        let factory = factory(vec![instance(InstanceStatus {
            applied_revision: Some(revision('b').to_string()),
            serving_revision: Some(revision('a').to_string()),
            failed: Some(Failure {
                stage: "image".to_owned(),
                message: Some("not found".to_owned()),
            }),
            ..Default::default()
        })]);
        let found = running(&factory, &repository(), "main", &revision('b')).unwrap();
        assert!(!found.serving);
        assert_eq!(found.failed_stage.as_deref(), Some("image"));
        assert_eq!(found.still_serving, Some(revision('a')));
    }

    #[test]
    fn a_linked_revision_says_the_link_and_the_branch_it_serves_on() {
        let factory = factory(vec![instance(InstanceStatus {
            serving_revision: Some(revision('c').to_string()),
            runner: Some(RunnerStatus {
                link: "team-a".to_owned(),
                linked_repository: OTHER.to_owned(),
                tracked_branch: TrackedBranch {
                    name: "main-tracked".to_owned(),
                },
                linked_revision: Some(revision('d').to_string()),
            }),
            ..Default::default()
        })]);
        let linked = RepositoryId::parse(OTHER).unwrap();
        let found = running(&factory, &linked, "main-tracked", &revision('d')).unwrap();
        assert!(found.serving);
        assert_eq!(found.link.as_deref(), Some("team-a"));
        assert_eq!(found.on.as_deref(), Some("main"));
        assert_eq!(
            found.instance_repository.as_ref().map(RepositoryId::as_str),
            Some(REPOSITORY)
        );
        assert_eq!(
            running(&factory, &repository(), "main", &revision('c')),
            None
        );
    }

    #[test]
    fn a_push_is_yours_when_you_are_who_lore_names() {
        let place = Place {
            repository: repository(),
            name: "n8n".to_owned(),
            branch: Branch {
                id: "b".to_owned(),
                name: "main".to_owned(),
                latest: None,
                forked_at: None,
            },
        };
        let record = |principal: &str| RevisionRecord {
            revision: revision('a'),
            branch_id: Some("b".to_owned()),
            number: 1,
            message: "m".to_owned(),
            at: 5,
            by: Committer {
                principal: Some(principal.to_owned()),
                name: None,
                email: None,
            },
            parent: None,
            merged: None,
        };
        let factory = factory(Vec::new());
        assert!(
            push_of(&place, &record("me"), Some(&factory))
                .unwrap()
                .by
                .yours
        );
        assert!(
            !push_of(&place, &record("you"), Some(&factory))
                .unwrap()
                .by
                .yours
        );
    }
}
