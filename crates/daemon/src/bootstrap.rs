//! The Nexus bootstrap every Nexus starts with (`nexus_serving::NexusBootstrap`)
//! names the Place, the admission and the paired translator; the Belief
//! Nexus checks it before it has any effect.
//!
//! The sockets and the memory file are no part of it: they are named after
//! the Nexus in the Place (`nexus_exchange::SocketPaths` and
//! `nexus_exchange::MemoryPlacing`). The owner of the Nexus's state is the
//! login it runs as, which the paired translator must run as too.

use nexus_serving::NexusBootstrap;
use signal_belief_nexus as wire;

/// Why a Nexus bootstrap is refused before it has any effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BootstrapRefusal {
    UnsafePlaceName(String),
    RelativePlace(String),
    UnnormalizedPlace(String),
    PairedUserNotEffectiveUser,
    InvalidAdmission,
}

/// A bootstrap that names a prepared Place, checked before any effect.
pub trait Validating {
    fn validated(&self, effective_user: i64) -> Result<(), BootstrapRefusal>;
}

/// The owner of the state a bootstrap starts: the login of its paired
/// translator, which [`Validating`] requires to be the effective user.
pub trait Owning {
    fn owner(&self) -> wire::Owner;
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
        translator::placement::validate_place_name(&self.place.place_name.0)
            .map_err(|_| BootstrapRefusal::UnsafePlaceName(self.place.place_name.0.clone()))?;
        for path in [&self.place.runtime.0, &self.place.state.0] {
            if !std::path::Path::new(path).is_absolute() {
                return Err(BootstrapRefusal::RelativePlace(path.clone()));
            }
            if !normalized_absolute(path) {
                return Err(BootstrapRefusal::UnnormalizedPlace(path.clone()));
            }
        }
        if self.peer_identity.user_id.0 != effective_user {
            return Err(BootstrapRefusal::PairedUserNotEffectiveUser);
        }
        Ok(())
    }
}

impl Owning for NexusBootstrap {
    fn owner(&self) -> wire::Owner {
        wire::Owner::Login(self.peer_identity.user_id)
    }
}
