//! The Belief Nexus's own wire: its call contracts, its bootstrap and its
//! name. The exchange layer, the lanes and the admission they share are every
//! Nexus's, in `nexus-exchange`.
//!
//! The query of every exchange is a Call with its Caller: `public_call::Input`
//! on the public socket and `meta_call::Input` on the meta socket, each
//! greeting with the digest of its own authored source. The kinds declared in
//! `ethos/` are generated into `src/generated/` by ethos-zero; the flake's
//! check refuses a committed generation that differs from what the source
//! generates.

pub mod bootstrap;
pub mod exchange;
#[rustfmt::skip]
#[path = "generated/meta_call.rs"]
pub mod meta_call;
#[rustfmt::skip]
#[path = "generated/public_call.rs"]
pub mod public_call;
#[rustfmt::skip]
#[path = "generated/transport.rs"]
pub mod transport;

pub use nexus_exchange::ValidAdmission;

use nexus_exchange::{ComponentName, ComponentTitle, Named, SocketNaming};

/// The Belief Nexus, which its sockets and its memory file are named after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeliefNexus;

impl Named for BeliefNexus {
    fn name() -> ComponentName {
        ComponentName("belief".into())
    }

    fn title() -> ComponentTitle {
        ComponentTitle("Belief Nexus".into())
    }
}

/// `belief.sock` and `belief-meta.sock` are the Nexus's own;
/// `belief-translator.sock` and `belief-meta-translator.sock` its
/// translator's.
pub fn socket_names() -> nexus_exchange::SocketNames {
    BeliefNexus::name().socket_names()
}

/// The Nexus's memory file in its place's state directory: `belief.memory`.
pub fn memory_file_name() -> transport::MemoryFileName {
    transport::MemoryFileName(format!("{}.memory", BeliefNexus::name().0))
}
