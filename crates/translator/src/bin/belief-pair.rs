use belief_nexus_translator::{deployment_from_text, place_from_text, Deployment};
use meta_signal_belief_nexus as meta;
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::waitpid;
use nix::unistd::{Pid, Uid};
use shared_translator::{ByteViewable, FrameBody, FrameCapacity, FrameWriting, ProcessId, Signalizable, UserId};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[derive(Debug)]
struct MemoryFiles {
    current: PathBuf,
    legacy: PathBuf,
}

trait MemoryFilesFromCurrent {
    fn from_current(current: PathBuf) -> Self;
}

trait MemoryFileSelecting {
    fn select(self) -> Result<PathBuf, MemoryFileSelectionError>;
}

#[derive(Debug)]
enum MemoryFileSelectionError {
    Inspection { path: PathBuf, source: std::io::Error },
    Conflict { current: PathBuf, legacy: PathBuf },
    Migration { current: PathBuf, legacy: PathBuf, source: std::io::Error },
}

impl MemoryFilesFromCurrent for MemoryFiles {
    fn from_current(current: PathBuf) -> Self {
        let legacy = current.with_extension("sema");
        Self { current, legacy }
    }
}

impl MemoryFileSelecting for MemoryFiles {
    fn select(self) -> Result<PathBuf, MemoryFileSelectionError> {
        let current_exists = self.current.try_exists().map_err(|source| MemoryFileSelectionError::Inspection {
            path: self.current.clone(),
            source,
        })?;
        let legacy_exists = self.legacy.try_exists().map_err(|source| MemoryFileSelectionError::Inspection {
            path: self.legacy.clone(),
            source,
        })?;
        match (current_exists, legacy_exists) {
            (true, true) => Err(MemoryFileSelectionError::Conflict { current: self.current, legacy: self.legacy }),
            (false, true) => {
                std::fs::rename(&self.legacy, &self.current).map_err(|source| MemoryFileSelectionError::Migration {
                    current: self.current.clone(),
                    legacy: self.legacy.clone(),
                    source,
                })?;
                Ok(self.current)
            }
            (true, false) | (false, false) => Ok(self.current),
        }
    }
}

impl std::fmt::Display for MemoryFileSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inspection { path, source } => write!(formatter, "could not inspect Memory path {}: {source}", path.display()),
            Self::Conflict { current, legacy } => write!(
                formatter,
                "Memory path conflict: current {} and legacy {} both exist",
                current.display(),
                legacy.display()
            ),
            Self::Migration { current, legacy, source } => write!(
                formatter,
                "could not migrate legacy Memory {} to current {}: {source}",
                legacy.display(),
                current.display()
            ),
        }
    }
}

impl std::error::Error for MemoryFileSelectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Inspection { source, .. } | Self::Migration { source, .. } => Some(source),
            Self::Conflict { .. } => None,
        }
    }
}

fn component(value: &str) -> Result<&str, Box<dyn std::error::Error>> {
    let mut parts = Path::new(value).components();
    if matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none() { Ok(value) } else { Err(format!("deployment name is not one path component: {value}").into()) }
}
fn stop(child: &mut Child) {
    let pid = Pid::from_raw(i32::try_from(child.id()).expect("child pid fits i32"));
    let _ = kill(pid, Signal::SIGTERM); let _ = waitpid(pid, None);
}
fn socket(root: &Path, name: &belief_nexus_translator::SocketName) -> Result<PathBuf, Box<dyn std::error::Error>> { Ok(root.join(component(&name.0)?)) }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let place = place_from_text(&arguments.next().ok_or("PLACE")?)?;
    let deployment: Deployment = deployment_from_text(&arguments.next().ok_or("DEPLOYMENT")?)?;
    if arguments.next().is_some() { return Err("unexpected launcher argument".into()); }
    shared_translator::placement::validate_metaflow_name(&place.metaflow_name.0)?;
    let runtime = shared_translator::placement::runtime_path(&place);
    let state = shared_translator::placement::state_path(&place);
    if !runtime.is_dir() { return Err(format!("metaflow runtime does not exist: {}", runtime.display()).into()); }
    if !state.is_dir() { return Err(format!("metaflow state does not exist: {}", state.display()).into()); }
    let ordinary = socket(&runtime, &deployment.ordinary_socket_name)?;
    let privileged = socket(&runtime, &deployment.meta_socket_name)?;
    let translator_socket = socket(&runtime, &deployment.ordinary_translator_socket_name)?;
    let meta_translator = socket(&runtime, &deployment.meta_translator_socket_name)?;
    let memory = MemoryFiles::from_current(state.join(component(&deployment.memory_file_name.0)?)).select()?;

    let mut nexus = Command::new(&deployment.nexus_executable.0).stdin(Stdio::piped()).spawn()?;
    let mut translator = match Command::new(&deployment.translator_executable.0).stdin(Stdio::piped()).spawn() {
        Ok(child) => child, Err(error) => { stop(&mut nexus); return Err(error.into()); }
    };
    let uid = UserId(i64::from(Uid::current().as_raw()));
    let gid = shared_translator::GroupId(i64::from(nix::unistd::Gid::current().as_raw()));
    if deployment.owner != signal_belief_nexus::Owner::Login(uid.clone()) { return Err("deployment owner is not the launching login".into()); }
    let capacity = FrameCapacity::from(signal::MAXIMUM_SIGNAL_BYTES);
    let bootstrapped = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut input = nexus.stdin.take().ok_or("Nexus bootstrap pipe")?;
        let bootstrap = meta::NexusBootstrap {
            ordinary_socket: meta::OrdinarySocket(meta::SocketPath(ordinary.to_string_lossy().into_owned())),
            meta_socket: meta::MetaSocket(meta::SocketPath(privileged.to_string_lossy().into_owned())),
            emission_translator_socket: meta::EmissionTranslatorSocket(meta::SocketPath(translator_socket.to_string_lossy().into_owned())),
            memory_path: meta::MemoryPath(memory.to_string_lossy().into_owned()), admission: deployment.admission.clone(),
            paired_translator: meta::PairedTranslator { user_id: uid.clone(), group_id: gid.clone(), process_id: ProcessId(i64::from(translator.id())) },
            bootstrap_owner: meta::BootstrapOwner(deployment.owner.clone()),
        };
        let archived = bootstrap.signalize()?;
        input.write_frame(&FrameBody::from(archived.bytes().to_vec()), capacity)?; input.flush()?; drop(input);

        let mut input = translator.stdin.take().ok_or("translator bootstrap pipe")?;
        let bootstrap = meta::TranslatorBootstrap {
            ordinary_translator_socket: meta::OrdinaryTranslatorSocket(meta::SocketPath(translator_socket.to_string_lossy().into_owned())),
            meta_translator_socket: meta::MetaTranslatorSocket(meta::SocketPath(meta_translator.to_string_lossy().into_owned())),
            ordinary_nexus_socket: meta::OrdinaryNexusSocket(meta::SocketPath(ordinary.to_string_lossy().into_owned())),
            meta_nexus_socket_path: meta::MetaNexusSocketPath(meta::SocketPath(privileged.to_string_lossy().into_owned())),
            admission: deployment.admission, paired_process: meta::PairedProcess { user_id: uid, group_id: gid, process_id: ProcessId(i64::from(nexus.id())) },
        };
        let archived = bootstrap.signalize()?;
        input.write_frame(&FrameBody::from(archived.bytes().to_vec()), capacity)?; input.flush()?; drop(input); Ok(())
    })();
    if let Err(error) = bootstrapped { stop(&mut nexus); stop(&mut translator); return Err(error); }
    for _ in 0..700 {
        if meta_translator.exists() { break; }
        std::thread::sleep(Duration::from_millis(10));
    }
    if !meta_translator.exists() { stop(&mut nexus); stop(&mut translator); return Err("meta translator socket did not appear".into()); }
    let mut sets = deployment.set_vector;
    sets.extend([
        meta::Set::NexusSocket(meta::SocketPath(ordinary.to_string_lossy().into_owned())),
        meta::Set::MetaNexusSocket(meta::SocketPath(privileged.to_string_lossy().into_owned())),
        meta::Set::AllowedNexusPeers(signal_belief_nexus::PeerList(vec![shared_translator::PeerBinding {
            peer_id: shared_translator::PeerId("belief-nexus".into()),
            peer_identity: shared_translator::PeerIdentity { user_id: UserId(i64::from(Uid::current().as_raw())), process_id: ProcessId(i64::from(nexus.id())) },
        }])),
    ]);
    for (revision, set) in sets.into_iter().enumerate() {
        let query = meta::Query::Configure(meta::ConfigureRequest { configuration_revision_option: Some(meta::ConfigurationRevision(revision as i64)), set });
        let text = belief_nexus_translator::text_request(&meta_translator, &belief_nexus_translator::encode_meta_query(&query), capacity)?;
        let response = belief_nexus_translator::decode_meta_response_text(&text)
            .map_err(|error| format!("deployment response was not meta signal: {text}: {error}"))?;
        match response {
            meta::Response::Configured(_) => {}
            meta::Response::ConfigureRefused(meta::ConfigurationRefusal::StaleRevision(_)) if revision == 0 => break,
            other => { stop(&mut nexus); stop(&mut translator); return Err(format!("deployment configuration failed: {other:?}").into()); }
        }
    }
    let nexus_pid = Pid::from_raw(i32::try_from(nexus.id()).expect("child pid fits i32"));
    let translator_pid = Pid::from_raw(i32::try_from(translator.id()).expect("child pid fits i32"));
    let ended = waitpid(Pid::from_raw(-1), None)?;
    let survivor = if ended.pid() == Some(nexus_pid) { translator_pid } else { nexus_pid };
    let _ = kill(survivor, Signal::SIGTERM); let _ = waitpid(survivor, None);
    Err(format!("paired Belief process ended: {ended:?}").into())
}

#[cfg(test)]
mod tests {
    use super::{MemoryFileSelecting, MemoryFileSelectionError, MemoryFiles, MemoryFilesFromCurrent};

    trait MemoryFixture {
        fn memory_files(&self) -> MemoryFiles;
    }

    impl MemoryFixture for tempfile::TempDir {
        fn memory_files(&self) -> MemoryFiles {
            MemoryFiles::from_current(self.path().join("belief.memory"))
        }
    }

    #[test]
    fn legacy_only_memory_is_migrated_to_the_current_path() {
        let directory = tempfile::tempdir().unwrap();
        let files = directory.memory_files();
        std::fs::write(&files.legacy, b"memory").unwrap();
        let selected = files.select().unwrap();
        assert_eq!(selected, directory.path().join("belief.memory"));
        assert_eq!(std::fs::read(selected).unwrap(), b"memory");
        assert!(!directory.path().join("belief.sema").exists());
    }

    #[test]
    fn current_only_memory_is_used_without_change() {
        let directory = tempfile::tempdir().unwrap();
        let files = directory.memory_files();
        std::fs::write(&files.current, b"current").unwrap();
        let selected = files.select().unwrap();
        assert_eq!(std::fs::read(selected).unwrap(), b"current");
    }

    #[test]
    fn current_and_legacy_memory_are_a_typed_conflict_naming_both_paths() {
        let directory = tempfile::tempdir().unwrap();
        let files = directory.memory_files();
        std::fs::write(&files.current, b"current").unwrap();
        std::fs::write(&files.legacy, b"legacy").unwrap();
        let current = files.current.clone();
        let legacy = files.legacy.clone();
        let error = files.select().unwrap_err();
        match &error {
            MemoryFileSelectionError::Conflict { current: found_current, legacy: found_legacy } => {
                assert_eq!(found_current, &current);
                assert_eq!(found_legacy, &legacy);
            }
            other => panic!("expected typed Memory path conflict, got {other:?}"),
        }
        let message = error.to_string();
        assert!(message.contains(current.to_str().unwrap()));
        assert!(message.contains(legacy.to_str().unwrap()));
        assert_eq!(std::fs::read(current).unwrap(), b"current");
        assert_eq!(std::fs::read(legacy).unwrap(), b"legacy");
    }

    #[test]
    fn fresh_memory_selects_the_current_path_without_creating_it() {
        let directory = tempfile::tempdir().unwrap();
        let selected = directory.memory_files().select().unwrap();
        assert_eq!(selected, directory.path().join("belief.memory"));
        assert!(!selected.exists());
    }
}
