//! The Nexus bootstrap names a Place, its memory file, its owner and its
//! paired translator, and is checked before it has any effect.
//!
//! The sockets are no part of it: the Nexus's four sockets are named after
//! the Nexus in the Place's runtime directory (`nexus_exchange::SocketPaths`).

use crate::transport::NexusBootstrap;

/// Why a Nexus bootstrap is refused before it has any effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BootstrapRefusal {
    UnsafePlaceName(String),
    RelativePlace(String),
    UnnormalizedPlace(String),
    UnsafeFileName(String),
    LegacyExtension(String),
    OwnerNotEffectiveUser,
    PairedUserNotEffectiveUser,
    InvalidAdmission,
}

/// A bootstrap that names a prepared Place, checked before any effect.
pub trait Validating {
    fn validated(&self, effective_user: i64) -> Result<(), BootstrapRefusal>;
}

/// The one memory path a bootstrap names: the Place state joined with the
/// file name.
pub trait Locating {
    fn memory_path(&self) -> std::path::PathBuf;
}

/// One raw path component: exactly one normal component equal to the text,
/// no NUL. The one rule for every file name a bootstrap carries.
pub trait Component {
    fn is_one_component(&self) -> bool;
}

impl Component for str {
    fn is_one_component(&self) -> bool {
        let mut parts = std::path::Path::new(self).components();
        matches!(parts.next(), Some(std::path::Component::Normal(part)) if part == self)
            && parts.next().is_none()
            && !self.contains('\0')
    }
}

fn normalized_absolute(value: &str) -> bool {
    value.starts_with('/')
        && !value.contains('\0')
        && (value == "/"
            || value[1..]
                .split('/')
                .all(|segment| !segment.is_empty() && segment != "." && segment != ".."))
}

impl Validating for NexusBootstrap {
    fn validated(&self, effective_user: i64) -> Result<(), BootstrapRefusal> {
        nexus_exchange::ValidAdmission::try_from(&self.admission)
            .map_err(|_| BootstrapRefusal::InvalidAdmission)?;
        shared_translator::placement::validate_place_name(&self.place.place_name.0)
            .map_err(|_| BootstrapRefusal::UnsafePlaceName(self.place.place_name.0.clone()))?;
        for path in [&self.place.runtime.0, &self.place.state.0] {
            if !std::path::Path::new(path).is_absolute() {
                return Err(BootstrapRefusal::RelativePlace(path.clone()));
            }
            if !normalized_absolute(path) {
                return Err(BootstrapRefusal::UnnormalizedPlace(path.clone()));
            }
        }
        if !self.memory_file_name.0.is_one_component() {
            return Err(BootstrapRefusal::UnsafeFileName(
                self.memory_file_name.0.clone(),
            ));
        }
        if std::path::Path::new(&self.memory_file_name.0)
            .with_extension("sema")
            .as_os_str()
            == std::ffi::OsStr::new(&self.memory_file_name.0)
        {
            return Err(BootstrapRefusal::LegacyExtension(
                self.memory_file_name.0.clone(),
            ));
        }
        match &self.owner {
            signal_belief_nexus::Owner::Login(user) if user.0 == effective_user => {}
            _ => return Err(BootstrapRefusal::OwnerNotEffectiveUser),
        }
        if self.peer_identity.user_id.0 != effective_user {
            return Err(BootstrapRefusal::PairedUserNotEffectiveUser);
        }
        Ok(())
    }
}

impl Locating for NexusBootstrap {
    fn memory_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.place.state.0).join(&self.memory_file_name.0)
    }
}
