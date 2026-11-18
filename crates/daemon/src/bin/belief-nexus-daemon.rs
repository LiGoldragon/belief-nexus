use nexus_serving::{BootstrapReading, NexusBootstrap};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = {
        let mut input = std::io::stdin().lock();
        let bootstrap = NexusBootstrap::read_bootstrap(&mut input)?;
        drop(input);
        bootstrap
    };
    nix::unistd::close(0)?;
    belief_nexus_daemon::server::run(bootstrap)
}
