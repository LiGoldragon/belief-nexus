//! Pure domain state for Belief.
//!
//! Source acquisition and filesystem publication are effects.  This crate
//! consumes complete, immutable source snapshots and produces complete,
//! immutable workspace images; it never reads a checkout or edits a workspace.

use emission_belief_nexus as emission;
use meta_signal_belief_nexus as meta;
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
pub enum PublicationPlanError {
    StalePlan,
    InvalidLayout(wire::RelativePath),
    ConflictingDestinations(wire::RelativePath, wire::RelativePath),
    NonTextModule(wire::ModuleName),
    RevisionExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionError {
    NoPendingPublication,
    PublicationIdentityMismatch,
    RecoveryEvidenceMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryOutcome {
    Finalized(wire::FurnishedWorkspace),
    Retry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationContext {
    pub owner: wire::Caller,
    pub configuration_revision: meta::ConfigurationRevision,
    pub layout: meta::Layout,
    pub retention_policy: meta::RetentionPolicy,
}

/// The complete intent that Memory must durably store before Publish is emitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingIntent {
    pub owner: wire::Caller,
    pub harness: wire::Harness,
    pub configuration_revision: meta::ConfigurationRevision,
    pub requested: Vec<wire::ModuleName>,
    pub publication: emission::Publication,
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
    pending: Option<PendingPublication>,
}

#[derive(Clone, Debug)]
struct PendingPublication {
    intent: PendingIntent,
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
                pending: None,
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
        let source_revisions = all_revisions(&self.repositories);
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

    /// Persists the exact publication intent in domain state before returning the
    /// effect.  The caller must durably record this state before it emits Publish.
    pub fn begin_publication(
        &mut self,
        plan: &FurnishPlan,
        context: PublicationContext,
    ) -> Result<emission::Publication, PublicationPlanError> {
        validate_layout(&context.layout)?;
        if !self.plan_is_current(plan) {
            return Err(PublicationPlanError::StalePlan);
        }

        let prior = self
            .workspaces
            .get(&plan.workspace)
            .and_then(|record| record.successful.as_ref())
            .map(|successful| successful.publication_revision.clone());
        let intended = wire::PublicationRevision(
            prior
                .as_ref()
                .map_or(Ok(1), |revision| revision.0.checked_add(1).ok_or(()))
                .map_err(|()| PublicationPlanError::RevisionExhausted)?,
        );
        let publication = emission::Publication {
            workspace: plan.workspace.clone(),
            generation_id: plan.generation_id.clone(),
            prior_publication: emission::PriorPublication(prior),
            intended_publication: emission::IntendedPublication(intended),
            workspace_image: emission_image(&plan.image, context.layout.clone())?,
            repository_states: emission::RepositoryStates(
                plan.source_revisions
                    .iter()
                    .map(|(repository, source_revision)| wire::RepositoryState {
                        repository: repository.clone(),
                        source_revision: source_revision.clone(),
                    })
                    .collect(),
            ),
            found_modules: emission::FoundModules(
                plan.found.iter().map(|found| found.0.clone()).collect(),
            ),
            missing_modules: emission::MissingModules(plan.missing.clone()),
            retention_policy: context.retention_policy,
        };
        self.workspaces
            .get_mut(&plan.workspace)
            .expect("a current furnish plan has a workspace record")
            .pending = Some(PendingPublication {
            intent: PendingIntent {
                owner: context.owner,
                harness: plan.harness.clone(),
                configuration_revision: context.configuration_revision,
                requested: plan.requested.clone(),
                publication: publication.clone(),
            },
        });
        Ok(publication)
    }

    pub fn pending_intent(&self, workspace: &wire::Workspace) -> Option<&PendingIntent> {
        Some(&self.workspaces.get(workspace)?.pending.as_ref()?.intent)
    }

    /// Accepts an effect result only when it names the exact pending publication.
    pub fn complete_publication(
        &mut self,
        state: &emission::PublicationState,
    ) -> Result<wire::FurnishedWorkspace, CompletionError> {
        let pending = self
            .workspaces
            .get(&state.workspace)
            .and_then(|record| record.pending.clone())
            .ok_or(CompletionError::NoPendingPublication)?;
        if state.generation_id != pending.intent.publication.generation_id
            || state.publication_revision != pending.intent.publication.intended_publication.0
        {
            return Err(CompletionError::PublicationIdentityMismatch);
        }
        self.commit_pending(&state.workspace)
    }

    /// Recovers a publication whose pointer was replaced before Memory recorded
    /// success.  Pointer and manifest are independent evidence and both must match
    /// every identity-bearing field of the pending intent.
    pub fn recover_publication(
        &mut self,
        inspection: &emission::PublicationInspection,
    ) -> Result<RecoveryOutcome, CompletionError> {
        let pending = self
            .workspaces
            .get(&inspection.workspace)
            .and_then(|record| record.pending.as_ref())
            .ok_or(CompletionError::NoPendingPublication)?;
        let expected_pointer = emission::OwnedPointer {
            generation_id: pending.intent.publication.generation_id.clone(),
            publication_revision: pending.intent.publication.intended_publication.0.clone(),
        };
        let expected_manifest = manifest_of(&pending.intent.publication);
        if inspection.owned_pointer_option.as_ref() == Some(&expected_pointer) {
            if inspection.publication_manifest_option.as_ref() != Some(&expected_manifest) {
                return Err(CompletionError::RecoveryEvidenceMismatch);
            }
            return self
                .commit_pending(&inspection.workspace)
                .map(RecoveryOutcome::Finalized);
        }
        let prior_pointer = self
            .workspaces
            .get(&inspection.workspace)
            .and_then(|record| record.successful.as_ref())
            .map(|successful| emission::OwnedPointer {
                generation_id: successful.generation_id.clone(),
                publication_revision: successful.publication_revision.clone(),
            });
        if inspection.owned_pointer_option == prior_pointer {
            Ok(RecoveryOutcome::Retry)
        } else {
            Err(CompletionError::RecoveryEvidenceMismatch)
        }
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
        if self.resolve_sources(&record.requested).is_err() {
            return true;
        }
        let current = all_revisions(&self.repositories);
        successful.source_revisions != current
            || record.last_found.len() + record.last_missing.len() != record.requested.len()
            || self.resolve(&record.requested).is_ok_and(|resolution| {
                resolution.found_vector != record.last_found
                    || resolution.module_name_vector != record.last_missing
            })
    }

    fn plan_is_current(&self, plan: &FurnishPlan) -> bool {
        let Some(record) = self.workspaces.get(&plan.workspace) else {
            return false;
        };
        if record.harness != plan.harness || record.requested != plan.requested {
            return false;
        }
        let Ok(resolved) = self.resolve_sources(&plan.requested) else {
            return false;
        };
        let image = build_image(&resolved);
        plan.image == image
            && plan.generation_id == generation_id(&plan.workspace, &image)
            && plan.source_revisions == all_revisions(&self.repositories)
    }

    fn commit_pending(
        &mut self,
        workspace: &wire::Workspace,
    ) -> Result<wire::FurnishedWorkspace, CompletionError> {
        let record = self
            .workspaces
            .get_mut(workspace)
            .ok_or(CompletionError::NoPendingPublication)?;
        let pending = record
            .pending
            .take()
            .ok_or(CompletionError::NoPendingPublication)?;
        if record.requested != pending.intent.requested || record.harness != pending.intent.harness
        {
            record.pending = Some(pending);
            return Err(CompletionError::PublicationIdentityMismatch);
        }
        let publication = pending.intent.publication;
        let found_vector: Vec<_> = publication
            .found_modules
            .0
            .iter()
            .cloned()
            .map(wire::Found)
            .collect();
        let missing = publication.missing_modules.0.clone();
        record.successful = Some(SuccessfulPublication {
            generation_id: publication.generation_id.clone(),
            publication_revision: publication.intended_publication.0.clone(),
            source_revisions: publication
                .repository_states
                .0
                .iter()
                .map(|state| (state.repository.clone(), state.source_revision.clone()))
                .collect(),
        });
        record.last_found = found_vector.clone();
        record.last_missing = missing.clone();
        Ok(wire::FurnishedWorkspace {
            workspace: workspace.clone(),
            generation_id: publication.generation_id,
            publication_revision: publication.intended_publication.0,
            found_vector,
            module_name_vector: missing,
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

impl From<emission::SourceSnapshot> for SourceSnapshot {
    fn from(snapshot: emission::SourceSnapshot) -> Self {
        Self {
            repository: snapshot.repository,
            revision: snapshot.source_revision,
            modules: snapshot
                .module_source_vector
                .into_iter()
                .map(|module| ModuleSource {
                    name: module.module_name,
                    relative_path: module.relative_path,
                    bytes: module.content.0.into_bytes(),
                })
                .collect(),
        }
    }
}

fn emission_image(
    image: &WorkspaceImage,
    layout: meta::Layout,
) -> Result<emission::WorkspaceImage, PublicationPlanError> {
    let content = String::from_utf8(image.model_instructions.clone()).map_err(|_| {
        PublicationPlanError::NonTextModule(wire::ModuleName {
            kind: wire::Kind::Spirit,
            subject: wire::Subject("bundle".into()),
        })
    })?;
    let mut skill_file_vector = Vec::with_capacity(image.skills.len());
    for skill in &image.skills {
        let content = String::from_utf8(skill.bytes.clone())
            .map_err(|_| PublicationPlanError::NonTextModule(skill.name.clone()))?;
        skill_file_vector.push(emission::SkillFile {
            relative_path: wire::RelativePath(format!(
                "{}-{}/SKILL.md",
                kind_name(&skill.name.kind),
                skill.name.subject.0
            )),
            content: emission::Content(content),
        });
    }
    Ok(emission::WorkspaceImage {
        layout,
        content: emission::Content(content),
        skill_file_vector,
    })
}

fn manifest_of(publication: &emission::Publication) -> emission::PublicationManifest {
    emission::PublicationManifest {
        generation_id: publication.generation_id.clone(),
        intended_publication: publication.intended_publication.clone(),
        repository_states: publication.repository_states.clone(),
        found_modules: publication.found_modules.clone(),
        missing_modules: publication.missing_modules.clone(),
    }
}

fn validate_layout(layout: &meta::Layout) -> Result<(), PublicationPlanError> {
    let paths = [
        &layout.owned_root.0,
        &layout.generations_directory.0,
        &layout.current_link.0,
        &layout.settings_file.0,
        &layout.instructions_file.0,
        &layout.skills_directory.0,
    ];
    for path in paths {
        if !safe_relative_path(&path.0) {
            return Err(PublicationPlanError::InvalidLayout(path.clone()));
        }
    }
    // These are the three independently owned workspace destinations.  Internal
    // generation/current/instruction names are relative to owned_root.
    let owned = &layout.owned_root.0 .0;
    let settings = &layout.settings_file.0 .0;
    let skills = &layout.skills_directory.0 .0;
    for (first, second) in [(owned, settings), (owned, skills), (settings, skills)] {
        if path_overlaps(first, second) {
            return Err(PublicationPlanError::ConflictingDestinations(
                wire::RelativePath(first.clone()),
                wire::RelativePath(second.clone()),
            ));
        }
    }
    let generations = &layout.generations_directory.0 .0;
    let current = &layout.current_link.0 .0;
    if path_overlaps(generations, current) {
        return Err(PublicationPlanError::ConflictingDestinations(
            wire::RelativePath(generations.clone()),
            wire::RelativePath(current.clone()),
        ));
    }
    Ok(())
}

fn safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\0')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn path_overlaps(first: &str, second: &str) -> bool {
    first == second
        || second
            .strip_prefix(first)
            .is_some_and(|rest| rest.starts_with('/'))
        || first
            .strip_prefix(second)
            .is_some_and(|rest| rest.starts_with('/'))
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

fn all_revisions(
    repositories: &HashMap<wire::Repository, RepositoryIndex>,
) -> Vec<(wire::Repository, wire::SourceRevision)> {
    let mut revisions: Vec<_> = repositories
        .iter()
        .map(|(repository, index)| (repository.clone(), index.revision.clone()))
        .collect();
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
    let mut names: Vec<_> = names
        .into_iter()
        .filter(|name| seen.insert(name.clone()))
        .collect();
    names.sort_by_key(module_key);
    names
}

fn module_key(name: &wire::ModuleName) -> String {
    let rank = match name.kind {
        wire::Kind::Spirit => 0,
        wire::Kind::Intent => 1,
        wire::Kind::Vision => 2,
        wire::Kind::Knowledge => 3,
        wire::Kind::Role => 4,
    };
    format!("{rank}:{}", name.subject.0)
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

    fn layout() -> meta::Layout {
        meta::Layout {
            owned_root: meta::OwnedRoot(wire::RelativePath(".belief".into())),
            generations_directory: meta::GenerationsDirectory(wire::RelativePath(
                "generations".into(),
            )),
            current_link: meta::CurrentLink(wire::RelativePath("current".into())),
            settings_file: meta::SettingsFile(wire::RelativePath(".codex/config.toml".into())),
            instructions_setting: meta::InstructionsSetting("model_instructions_file".into()),
            instructions_file: meta::InstructionsFile(wire::RelativePath(
                "model-instructions.md".into(),
            )),
            skills_directory: meta::SkillsDirectory(wire::RelativePath(".agents/skills".into())),
        }
    }

    fn publication_context() -> PublicationContext {
        PublicationContext {
            owner: wire::Caller {
                user_id: wire::UserId(1000),
                group_id: wire::GroupId(1000),
                process_id: wire::ProcessId(42),
                executable_path: wire::ExecutablePath("/nix/store/client".into()),
                contract_digest: wire::ContractDigest("sha256:contract".into()),
            },
            configuration_revision: meta::ConfigurationRevision(3),
            layout: layout(),
            retention_policy: meta::RetentionPolicy::KeepAll,
        }
    }

    fn publish(core: &mut BeliefCore, plan: &FurnishPlan) -> wire::FurnishedWorkspace {
        let publication = core.begin_publication(plan, publication_context()).unwrap();
        core.complete_publication(&emission::PublicationState {
            workspace: publication.workspace,
            generation_id: publication.generation_id,
            publication_revision: publication.intended_publication.0,
        })
        .unwrap()
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
        assert_eq!(plan.image.model_instructions, b"spirit\nintent\n");
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
        publish(&mut core, &first);

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
        let published = publish(&mut core, &first);
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
        assert_eq!(success.publication_revision, wire::PublicationRevision(1));
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

        let old_publication = core.begin_publication(&old, publication_context()).unwrap();
        let current = core.prepare_furnish(request("/w", vec![role])).unwrap();
        let current_publication = core
            .begin_publication(&current, publication_context())
            .unwrap();
        assert_eq!(
            core.complete_publication(&emission::PublicationState {
                workspace: old_publication.workspace,
                generation_id: old_publication.generation_id,
                publication_revision: old_publication.intended_publication.0,
            }),
            Err(CompletionError::PublicationIdentityMismatch)
        );
        assert!(core.successful_publication(&workspace("/w")).is_none());
        core.complete_publication(&emission::PublicationState {
            workspace: current_publication.workspace,
            generation_id: current_publication.generation_id,
            publication_revision: current_publication.intended_publication.0,
        })
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
        publish(&mut core, &plan);

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

    #[test]
    fn pending_intent_binds_owner_config_basis_request_and_image_before_publish() {
        let spirit = name(wire::Kind::Spirit, "core");
        let vision = name(wire::Kind::Vision, "nexus");
        let mut core = BeliefCore::new();
        core.observe(snapshot("/a", "a1", vec![(spirit.clone(), b"spirit")]))
            .unwrap();
        core.observe(snapshot("/b", "b1", vec![(vision.clone(), b"vision")]))
            .unwrap();
        let plan = core
            .prepare_furnish(request("/w", vec![vision.clone(), spirit.clone()]))
            .unwrap();
        let context = publication_context();
        let publication = core.begin_publication(&plan, context.clone()).unwrap();
        let pending = core.pending_intent(&workspace("/w")).unwrap();

        assert_eq!(pending.owner, context.owner);
        assert_eq!(
            pending.configuration_revision,
            context.configuration_revision
        );
        assert_eq!(pending.harness, wire::Harness::Codex);
        assert_eq!(pending.requested, vec![spirit, vision]);
        assert_eq!(pending.publication, publication);
        assert_eq!(
            publication.repository_states.0,
            vec![
                wire::RepositoryState {
                    repository: repo("/a"),
                    source_revision: wire::SourceRevision("a1".into())
                },
                wire::RepositoryState {
                    repository: repo("/b"),
                    source_revision: wire::SourceRevision("b1".into())
                }
            ]
        );
    }

    #[test]
    fn ownership_is_prevalidated_before_pending_or_prior_success_changes() {
        let vision = name(wire::Kind::Vision, "nexus");
        let mut core = BeliefCore::new();
        core.observe(snapshot("/r", "one", vec![(vision.clone(), b"one")]))
            .unwrap();
        let plan = core.prepare_furnish(request("/w", vec![vision])).unwrap();
        let mut context = publication_context();
        context.layout.settings_file =
            meta::SettingsFile(wire::RelativePath(".belief/config.toml".into()));
        assert!(matches!(
            core.begin_publication(&plan, context),
            Err(PublicationPlanError::ConflictingDestinations(_, _))
        ));
        assert!(core.pending_intent(&workspace("/w")).is_none());
        assert!(core.successful_publication(&workspace("/w")).is_none());
    }

    #[test]
    fn recovery_finalizes_only_exact_pointer_and_manifest_and_preserves_pending_otherwise() {
        let vision = name(wire::Kind::Vision, "nexus");
        let mut core = BeliefCore::new();
        core.observe(snapshot("/r", "one", vec![(vision.clone(), b"one")]))
            .unwrap();
        let plan = core.prepare_furnish(request("/w", vec![vision])).unwrap();
        let publication = core
            .begin_publication(&plan, publication_context())
            .unwrap();

        let retry = emission::PublicationInspection {
            workspace: workspace("/w"),
            owned_pointer_option: None,
            publication_manifest_option: None,
        };
        assert_eq!(
            core.recover_publication(&retry).unwrap(),
            RecoveryOutcome::Retry
        );
        assert!(core.pending_intent(&workspace("/w")).is_some());

        let mismatch = emission::PublicationInspection {
            workspace: workspace("/w"),
            owned_pointer_option: Some(emission::OwnedPointer {
                generation_id: publication.generation_id.clone(),
                publication_revision: publication.intended_publication.0.clone(),
            }),
            publication_manifest_option: None,
        };
        assert_eq!(
            core.recover_publication(&mismatch),
            Err(CompletionError::RecoveryEvidenceMismatch)
        );
        assert!(core.pending_intent(&workspace("/w")).is_some());

        let exact = emission::PublicationInspection {
            workspace: workspace("/w"),
            owned_pointer_option: Some(emission::OwnedPointer {
                generation_id: publication.generation_id.clone(),
                publication_revision: publication.intended_publication.0.clone(),
            }),
            publication_manifest_option: Some(manifest_of(&publication)),
        };
        assert!(matches!(
            core.recover_publication(&exact),
            Ok(RecoveryOutcome::Finalized(_))
        ));
        assert!(core.pending_intent(&workspace("/w")).is_none());
        assert_eq!(
            core.successful_publication(&workspace("/w"))
                .unwrap()
                .publication_revision,
            publication.intended_publication.0
        );
    }
}
