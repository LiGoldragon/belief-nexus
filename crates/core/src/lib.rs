//! Pure domain state for Belief.
//!
//! Source acquisition and filesystem publication are effects.  This crate
//! consumes complete, immutable source snapshots and produces complete,
//! immutable workspace images; it never reads a checkout or edits a workspace.

use sha2::{Digest, Sha256};
use signal_belief_nexus as wire;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleSource {
    pub name: wire::ModuleName,
    pub relative_path: wire::RelativePath,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSnapshot {
    pub repository: wire::Repository,
    pub revision: wire::SourceRevision,
    pub modules: Vec<ModuleSource>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotError {
    InvalidModulePath {
        name: wire::ModuleName,
        path: wire::RelativePath,
    },
    DuplicateModule {
        repository: wire::Repository,
        name: wire::ModuleName,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolveError {
    Ambiguous {
        name: wire::ModuleName,
        repositories: Vec<wire::Repository>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillAsset {
    pub name: wire::ModuleName,
    pub bytes: Vec<u8>,
}

/// A complete generation, independent of any filesystem layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceImage {
    pub model_instructions: Vec<u8>,
    pub skills: Vec<SkillAsset>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FurnishPlan {
    pub workspace: wire::Workspace,
    pub harness: wire::Harness,
    pub requested: Vec<wire::ModuleName>,
    pub found: Vec<wire::Found>,
    pub missing: Vec<wire::ModuleName>,
    pub image: WorkspaceImage,
    pub generation_id: wire::GenerationId,
    /// The observed revisions used to construct this exact image.
    pub source_revisions: Vec<(wire::Repository, wire::SourceRevision)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotChange {
    pub repository: wire::Repository,
    pub revision: wire::SourceRevision,
    pub changed: Vec<wire::ModuleName>,
    pub retry_workspaces: Vec<wire::Workspace>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuccessfulPublication {
    pub generation_id: wire::GenerationId,
    pub publication_revision: wire::PublicationRevision,
    pub source_revisions: Vec<(wire::Repository, wire::SourceRevision)>,
}

#[derive(Clone, Debug)]
struct IndexedModule {
    source: ModuleSource,
}

#[derive(Clone, Debug)]
struct RepositoryIndex {
    revision: wire::SourceRevision,
    modules: HashMap<wire::ModuleName, IndexedModule>,
}

#[derive(Clone, Debug)]
struct WorkspaceRecord {
    harness: wire::Harness,
    requested: Vec<wire::ModuleName>,
    successful: Option<SuccessfulPublication>,
    last_found: Vec<wire::Found>,
    last_missing: Vec<wire::ModuleName>,
}

/// Belief's in-memory domain state.
#[derive(Default)]
pub struct BeliefCore {
    repositories: HashMap<wire::Repository, RepositoryIndex>,
    workspaces: HashMap<wire::Workspace, WorkspaceRecord>,
}

impl BeliefCore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Atomically replaces one repository's observed index after validating the
    /// entire snapshot.  An invalid snapshot leaves the prior observation intact.
    pub fn observe(&mut self, snapshot: SourceSnapshot) -> Result<SnapshotChange, SnapshotError> {
        let replacement = validate_snapshot(&snapshot)?;
        let previous = self.repositories.get(&snapshot.repository);
        let changed = changed_names(previous, &replacement);
        let affected: HashSet<_> = changed.iter().cloned().collect();

        self.repositories
            .insert(snapshot.repository.clone(), replacement);

        let mut retry_workspaces: Vec<_> = self
            .workspaces
            .iter()
            .filter(|(_, record)| {
                record.requested.iter().any(|name| affected.contains(name))
                    || record.successful.as_ref().is_some_and(|successful| {
                        successful
                            .source_revisions
                            .iter()
                            .any(|(repository, revision)| {
                                repository == &snapshot.repository && revision != &snapshot.revision
                            })
                    })
            })
            .map(|(workspace, _)| workspace.clone())
            .collect();
        retry_workspaces.sort_by_key(workspace_key);

        Ok(SnapshotChange {
            repository: snapshot.repository,
            revision: snapshot.revision,
            changed,
            retry_workspaces,
        })
    }

    pub fn repository_states(&self) -> Vec<wire::RepositoryState> {
        let mut states: Vec<_> = self
            .repositories
            .iter()
            .map(|(repository, index)| wire::RepositoryState {
                repository: repository.clone(),
                source_revision: index.revision.clone(),
            })
            .collect();
        states.sort_by_key(|state| repository_key(&state.repository));
        states
    }

    pub fn resolve(
        &self,
        requested: &[wire::ModuleName],
    ) -> Result<wire::Resolution, ResolveError> {
        let resolved = self.resolve_sources(requested)?;
        Ok(wire::Resolution {
            found_vector: resolved
                .iter()
                .filter_map(|item| {
                    item.as_ref()
                        .map(|(_, module)| wire::Found(module.source.name.clone()))
                })
                .collect(),
            module_name_vector: requested
                .iter()
                .zip(resolved)
                .filter_map(|(name, item)| item.is_none().then_some(name.clone()))
                .collect(),
        })
    }

    /// Records the desired workspace request and returns a complete candidate.
    /// If resolution or later publication fails, the previous successful state is
    /// untouched and this request remains eligible for retry.
    pub fn prepare_furnish(
        &mut self,
        request: wire::FurnishRequest,
    ) -> Result<FurnishPlan, ResolveError> {
        let workspace = request.workspace.clone();
        let requested = deduplicate_preserving_order(request.module_name_vector);
        let record = self
            .workspaces
            .entry(workspace.clone())
            .or_insert_with(|| WorkspaceRecord {
                harness: request.harness.clone(),
                requested: Vec::new(),
                successful: None,
                last_found: Vec::new(),
                last_missing: Vec::new(),
            });
        record.harness = request.harness.clone();
        record.requested = requested.clone();

        let resolved = self.resolve_sources(&requested)?;
        let found: Vec<_> = resolved
            .iter()
            .filter_map(|item| {
                item.as_ref()
                    .map(|(_, module)| wire::Found(module.source.name.clone()))
            })
            .collect();
        let missing: Vec<_> = requested
            .iter()
            .zip(&resolved)
            .filter_map(|(name, item)| item.is_none().then_some(name.clone()))
            .collect();
        let image = build_image(&resolved);
        let source_revisions = revisions_used(&resolved, &self.repositories);
        let generation_id = generation_id(&workspace, &image);

        Ok(FurnishPlan {
            workspace,
            harness: request.harness,
            requested,
            found,
            missing,
            image,
            generation_id,
            source_revisions,
        })
    }

    /// Commits only a successfully published candidate.  A stale candidate cannot
    /// overwrite a newer desired request.
    pub fn publication_succeeded(
        &mut self,
        plan: &FurnishPlan,
        publication_revision: wire::PublicationRevision,
    ) -> Option<wire::FurnishedWorkspace> {
        let record = self.workspaces.get_mut(&plan.workspace)?;
        if record.harness != plan.harness || record.requested != plan.requested {
            return None;
        }
        record.successful = Some(SuccessfulPublication {
            generation_id: plan.generation_id.clone(),
            publication_revision: publication_revision.clone(),
            source_revisions: plan.source_revisions.clone(),
        });
        record.last_found = plan.found.clone();
        record.last_missing = plan.missing.clone();
        Some(wire::FurnishedWorkspace {
            workspace: plan.workspace.clone(),
            generation_id: plan.generation_id.clone(),
            publication_revision,
            found_vector: plan.found.clone(),
            module_name_vector: plan.missing.clone(),
        })
    }

    pub fn successful_publication(
        &self,
        workspace: &wire::Workspace,
    ) -> Option<&SuccessfulPublication> {
        self.workspaces.get(workspace)?.successful.as_ref()
    }

    pub fn requested(&self, workspace: &wire::Workspace) -> Option<&[wire::ModuleName]> {
        Some(&self.workspaces.get(workspace)?.requested)
    }

    pub fn retry_workspaces(&self) -> Vec<wire::Workspace> {
        let mut workspaces: Vec<_> = self
            .workspaces
            .iter()
            .filter(|(_, record)| self.needs_retry(record))
            .map(|(workspace, _)| workspace.clone())
            .collect();
        workspaces.sort_by_key(workspace_key);
        workspaces
    }

    fn needs_retry(&self, record: &WorkspaceRecord) -> bool {
        let Some(successful) = &record.successful else {
            return true;
        };
        let Ok(resolved) = self.resolve_sources(&record.requested) else {
            return true;
        };
        let current = revisions_used(&resolved, &self.repositories);
        successful.source_revisions != current
            || record.last_found.len() + record.last_missing.len() != record.requested.len()
            || self.resolve(&record.requested).is_ok_and(|resolution| {
                resolution.found_vector != record.last_found
                    || resolution.module_name_vector != record.last_missing
            })
    }

    fn resolve_sources<'a>(
        &'a self,
        requested: &[wire::ModuleName],
    ) -> Result<Vec<Option<(&'a wire::Repository, &'a IndexedModule)>>, ResolveError> {
        requested
            .iter()
            .map(|name| {
                let mut matches: Vec<_> = self
                    .repositories
                    .iter()
                    .filter_map(|(repository, index)| {
                        index.modules.get(name).map(|module| (repository, module))
                    })
                    .collect();
                matches.sort_by_key(|(repository, _)| repository_key(repository));
                match matches.len() {
                    0 => Ok(None),
                    1 => Ok(matches.pop()),
                    _ => Err(ResolveError::Ambiguous {
                        name: name.clone(),
                        repositories: matches
                            .into_iter()
                            .map(|(repository, _)| repository.clone())
                            .collect(),
                    }),
                }
            })
            .collect()
    }
}

fn validate_snapshot(snapshot: &SourceSnapshot) -> Result<RepositoryIndex, SnapshotError> {
    let mut modules = HashMap::new();
    for source in &snapshot.modules {
        if !valid_subject(&source.name.subject.0)
            || source.relative_path.0 != expected_module_path(&source.name)
        {
            return Err(SnapshotError::InvalidModulePath {
                name: source.name.clone(),
                path: source.relative_path.clone(),
            });
        }
        if modules
            .insert(
                source.name.clone(),
                IndexedModule {
                    source: source.clone(),
                },
            )
            .is_some()
        {
            return Err(SnapshotError::DuplicateModule {
                repository: snapshot.repository.clone(),
                name: source.name.clone(),
            });
        }
    }
    Ok(RepositoryIndex {
        revision: snapshot.revision.clone(),
        modules,
    })
}

fn valid_subject(subject: &str) -> bool {
    !subject.is_empty() && subject != "." && subject != ".." && !subject.contains(['/', '\\', '\0'])
}

fn expected_module_path(name: &wire::ModuleName) -> String {
    format!("{}/{}.md", kind_name(&name.kind), name.subject.0)
}

fn kind_name(kind: &wire::Kind) -> &'static str {
    match kind {
        wire::Kind::Spirit => "spirit",
        wire::Kind::Intent => "intent",
        wire::Kind::Vision => "vision",
        wire::Kind::Knowledge => "knowledge",
        wire::Kind::Role => "role",
    }
}

fn changed_names(
    previous: Option<&RepositoryIndex>,
    next: &RepositoryIndex,
) -> Vec<wire::ModuleName> {
    let mut names: HashSet<_> = next.modules.keys().cloned().collect();
    if let Some(previous) = previous {
        names.extend(previous.modules.keys().cloned());
        names.retain(
            |name| match (previous.modules.get(name), next.modules.get(name)) {
                (Some(old), Some(new)) => {
                    old.source.bytes != new.source.bytes
                        || old.source.relative_path != new.source.relative_path
                }
                _ => true,
            },
        );
    }
    let mut names: Vec<_> = names.into_iter().collect();
    names.sort_by_key(module_key);
    names
}

fn build_image(resolved: &[Option<(&wire::Repository, &IndexedModule)>]) -> WorkspaceImage {
    let mut prompt_modules = Vec::new();
    let mut skills = Vec::new();
    for (_, module) in resolved.iter().flatten() {
        match module.source.name.kind {
            wire::Kind::Spirit | wire::Kind::Intent => prompt_modules.push(&module.source),
            _ => skills.push(SkillAsset {
                name: module.source.name.clone(),
                bytes: module.source.bytes.clone(),
            }),
        }
    }
    prompt_modules.sort_by_key(|module| module_key(&module.name));
    skills.sort_by_key(|skill| module_key(&skill.name));

    let mut model_instructions = Vec::new();
    for module in prompt_modules {
        if !model_instructions.is_empty() && !model_instructions.ends_with(b"\n") {
            model_instructions.push(b'\n');
        }
        model_instructions.extend_from_slice(&module.bytes);
        if !model_instructions.ends_with(b"\n") {
            model_instructions.push(b'\n');
        }
    }
    WorkspaceImage {
        model_instructions,
        skills,
    }
}

fn revisions_used(
    resolved: &[Option<(&wire::Repository, &IndexedModule)>],
    repositories: &HashMap<wire::Repository, RepositoryIndex>,
) -> Vec<(wire::Repository, wire::SourceRevision)> {
    let mut seen = HashSet::new();
    let mut revisions = Vec::new();
    for (repository, _) in resolved.iter().flatten() {
        if seen.insert((*repository).clone()) {
            revisions.push((
                (*repository).clone(),
                repositories[*repository].revision.clone(),
            ));
        }
    }
    revisions.sort_by_key(|(repository, _)| repository_key(repository));
    revisions
}

fn generation_id(workspace: &wire::Workspace, image: &WorkspaceImage) -> wire::GenerationId {
    let mut digest = Sha256::new();
    hash_bytes(&mut digest, workspace.0 .0.as_bytes());
    hash_bytes(&mut digest, &image.model_instructions);
    for skill in &image.skills {
        hash_bytes(&mut digest, kind_name(&skill.name.kind).as_bytes());
        hash_bytes(&mut digest, skill.name.subject.0.as_bytes());
        hash_bytes(&mut digest, &skill.bytes);
    }
    wire::GenerationId(format!("sha256:{:x}", digest.finalize()))
}

fn hash_bytes(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
}

fn deduplicate_preserving_order(names: Vec<wire::ModuleName>) -> Vec<wire::ModuleName> {
    let mut seen = HashSet::new();
    names
        .into_iter()
        .filter(|name| seen.insert(name.clone()))
        .collect()
}

fn module_key(name: &wire::ModuleName) -> String {
    format!("{}:{}", kind_name(&name.kind), name.subject.0)
}

fn repository_key(repository: &wire::Repository) -> String {
    repository.0 .0.clone()
}

fn workspace_key(workspace: &wire::Workspace) -> String {
    workspace.0 .0.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(kind: wire::Kind, subject: &str) -> wire::ModuleName {
        wire::ModuleName {
            kind,
            subject: wire::Subject(subject.into()),
        }
    }

    fn repo(path: &str) -> wire::Repository {
        wire::Repository(wire::RepositoryPath(path.into()))
    }

    fn workspace(path: &str) -> wire::Workspace {
        wire::Workspace(wire::WorkspacePath(path.into()))
    }

    fn snapshot(
        repository: &str,
        revision: &str,
        modules: Vec<(wire::ModuleName, &[u8])>,
    ) -> SourceSnapshot {
        SourceSnapshot {
            repository: repo(repository),
            revision: wire::SourceRevision(revision.into()),
            modules: modules
                .into_iter()
                .map(|(name, bytes)| ModuleSource {
                    relative_path: wire::RelativePath(expected_module_path(&name)),
                    name,
                    bytes: bytes.to_vec(),
                })
                .collect(),
        }
    }

    fn request(workspace_path: &str, names: Vec<wire::ModuleName>) -> wire::FurnishRequest {
        wire::FurnishRequest {
            workspace: workspace(workspace_path),
            harness: wire::Harness::Codex,
            module_name_vector: names,
        }
    }

    #[test]
    fn module_path_is_exactly_kind_subject_md_and_snapshot_replacement_is_atomic() {
        let good = name(wire::Kind::Vision, "nexus");
        let mut core = BeliefCore::new();
        core.observe(snapshot("/r", "one", vec![(good.clone(), b"old")]))
            .unwrap();

        for bad in [
            "vision/nexus.md/extra",
            "../vision/nexus.md",
            "vision/deep/nexus.md",
            "intent/nexus.md",
        ] {
            let mut invalid = snapshot("/r", "two", vec![(good.clone(), b"new")]);
            invalid.modules[0].relative_path = wire::RelativePath(bad.into());
            assert!(matches!(
                core.observe(invalid),
                Err(SnapshotError::InvalidModulePath { .. })
            ));
            assert_eq!(
                core.repository_states()[0].source_revision,
                wire::SourceRevision("one".into())
            );
        }

        for subject in ["", ".", "..", "deep/nexus", "deep\\nexus"] {
            let unsafe_name = name(wire::Kind::Vision, subject);
            let invalid = snapshot("/r", "two", vec![(unsafe_name, b"new")]);
            assert!(matches!(
                core.observe(invalid),
                Err(SnapshotError::InvalidModulePath { .. })
            ));
        }
    }

    #[test]
    fn furnish_places_spirit_and_intent_in_bundle_and_every_other_kind_in_skills() {
        let spirit = name(wire::Kind::Spirit, "core");
        let intent = name(wire::Kind::Intent, "design");
        let vision = name(wire::Kind::Vision, "nexus");
        let mut core = BeliefCore::new();
        core.observe(snapshot(
            "/r",
            "one",
            vec![
                (vision.clone(), b"vision"),
                (intent.clone(), b"intent"),
                (spirit.clone(), b"spirit"),
            ],
        ))
        .unwrap();
        let plan = core
            .prepare_furnish(request("/w", vec![vision.clone(), spirit, intent]))
            .unwrap();
        assert_eq!(plan.image.model_instructions, b"intent\nspirit\n");
        assert_eq!(
            plan.image.skills,
            vec![SkillAsset {
                name: vision,
                bytes: b"vision".to_vec()
            }]
        );
        assert!(plan.missing.is_empty());
    }

    #[test]
    fn removed_and_restored_modules_recompute_from_persisted_request() {
        let vision = name(wire::Kind::Vision, "nexus");
        let mut core = BeliefCore::new();
        core.observe(snapshot("/r", "one", vec![(vision.clone(), b"one")]))
            .unwrap();
        let first = core
            .prepare_furnish(request("/w", vec![vision.clone()]))
            .unwrap();
        core.publication_succeeded(&first, wire::PublicationRevision(1))
            .unwrap();

        let removed = core.observe(snapshot("/r", "two", vec![])).unwrap();
        assert_eq!(removed.changed, vec![vision.clone()]);
        assert_eq!(removed.retry_workspaces, vec![workspace("/w")]);
        let absent = core
            .prepare_furnish(request(
                "/w",
                core.requested(&workspace("/w")).unwrap().to_vec(),
            ))
            .unwrap();
        assert_eq!(absent.missing, vec![vision.clone()]);
        assert!(absent.image.skills.is_empty());

        core.observe(snapshot("/r", "three", vec![(vision.clone(), b"three")]))
            .unwrap();
        let restored = core
            .prepare_furnish(request(
                "/w",
                core.requested(&workspace("/w")).unwrap().to_vec(),
            ))
            .unwrap();
        assert_eq!(restored.found, vec![wire::Found(vision)]);
        assert_eq!(restored.image.skills[0].bytes, b"three");
    }

    #[test]
    fn ambiguity_refuses_candidate_and_preserves_prior_success() {
        let vision = name(wire::Kind::Vision, "nexus");
        let mut core = BeliefCore::new();
        core.observe(snapshot("/a", "a1", vec![(vision.clone(), b"a")]))
            .unwrap();
        let first = core
            .prepare_furnish(request("/w", vec![vision.clone()]))
            .unwrap();
        let published = core
            .publication_succeeded(&first, wire::PublicationRevision(7))
            .unwrap();
        core.observe(snapshot("/b", "b1", vec![(vision.clone(), b"b")]))
            .unwrap();

        let error = core
            .prepare_furnish(request("/w", vec![vision.clone()]))
            .unwrap_err();
        assert_eq!(
            error,
            ResolveError::Ambiguous {
                name: vision,
                repositories: vec![repo("/a"), repo("/b")]
            }
        );
        let success = core.successful_publication(&workspace("/w")).unwrap();
        assert_eq!(success.generation_id, published.generation_id);
        assert_eq!(success.publication_revision, wire::PublicationRevision(7));
        assert_eq!(core.retry_workspaces(), vec![workspace("/w")]);
    }

    #[test]
    fn failed_or_stale_publication_never_advances_success_and_remains_retryable() {
        let vision = name(wire::Kind::Vision, "nexus");
        let role = name(wire::Kind::Role, "worker");
        let mut core = BeliefCore::new();
        core.observe(snapshot(
            "/r",
            "one",
            vec![(vision.clone(), b"one"), (role.clone(), b"role")],
        ))
        .unwrap();
        let old = core
            .prepare_furnish(request("/w", vec![vision.clone()]))
            .unwrap();
        assert_eq!(core.retry_workspaces(), vec![workspace("/w")]);
        assert!(core.successful_publication(&workspace("/w")).is_none());

        let current = core.prepare_furnish(request("/w", vec![role])).unwrap();
        assert!(core
            .publication_succeeded(&old, wire::PublicationRevision(1))
            .is_none());
        assert!(core.successful_publication(&workspace("/w")).is_none());
        core.publication_succeeded(&current, wire::PublicationRevision(2))
            .unwrap();
        assert!(core.retry_workspaces().is_empty());
    }

    #[test]
    fn observed_revision_advances_separately_from_successful_revision() {
        let vision = name(wire::Kind::Vision, "nexus");
        let mut core = BeliefCore::new();
        core.observe(snapshot("/r", "one", vec![(vision.clone(), b"one")]))
            .unwrap();
        let plan = core
            .prepare_furnish(request("/w", vec![vision.clone()]))
            .unwrap();
        core.publication_succeeded(&plan, wire::PublicationRevision(1))
            .unwrap();

        core.observe(snapshot("/r", "two", vec![(vision, b"two")]))
            .unwrap();
        assert_eq!(
            core.repository_states()[0].source_revision,
            wire::SourceRevision("two".into())
        );
        assert_eq!(
            core.successful_publication(&workspace("/w"))
                .unwrap()
                .source_revisions,
            vec![(repo("/r"), wire::SourceRevision("one".into()))]
        );
        assert_eq!(core.retry_workspaces(), vec![workspace("/w")]);
    }

    #[test]
    fn identical_images_have_workspace_unique_generation_identity() {
        let vision = name(wire::Kind::Vision, "nexus");
        let mut core = BeliefCore::new();
        core.observe(snapshot("/r", "one", vec![(vision.clone(), b"same")]))
            .unwrap();
        let first = core
            .prepare_furnish(request("/first", vec![vision.clone()]))
            .unwrap();
        let second = core
            .prepare_furnish(request("/second", vec![vision]))
            .unwrap();
        assert_eq!(first.image, second.image);
        assert_ne!(first.generation_id, second.generation_id);
    }
}
