//! Both Belief Nexus sockets speak `signal`'s exchange layer, which
//! `nexus-exchange` frames and `nexus-serving` serves. Each greets with the
//! digest of its own call contract's authored source.

use signal::Contracted;

/// The authored source of the public call contract, whose digest greets the
/// public socket.
pub const PUBLIC_CALL_ETHOS: &str = include_str!("../ethos/public-call.ethos");

/// The authored source of the meta call contract, whose digest greets the
/// meta socket.
pub const META_CALL_ETHOS: &str = include_str!("../ethos/meta-call.ethos");

impl Contracted for crate::public_call::Input {
    const CONTRACT_SOURCE: &'static str = PUBLIC_CALL_ETHOS;
}

impl Contracted for crate::meta_call::Input {
    const CONTRACT_SOURCE: &'static str = META_CALL_ETHOS;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_call_contracts_greet_with_different_digests() {
        assert_ne!(
            crate::public_call::Input::contract_digest(),
            crate::meta_call::Input::contract_digest()
        );
    }

    #[test]
    fn sockets_and_memory_keep_their_names() {
        let names = crate::socket_names();
        assert_eq!(names.public_nexus_socket.0, "belief.sock");
        assert_eq!(names.meta_nexus_socket.0, "belief-meta.sock");
        assert_eq!(names.public_translator_socket.0, "belief-translator.sock");
        assert_eq!(
            names.meta_translator_socket.0,
            "belief-meta-translator.sock"
        );
        assert_eq!(crate::memory_file().0, "belief.memory");
    }
}
