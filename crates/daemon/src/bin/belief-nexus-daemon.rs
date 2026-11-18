use signal::{FrameCapacity, FrameReading, Restorable, Signal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = {
        let body = std::io::stdin()
            .lock()
            .read_frame(FrameCapacity::from(signal::MAXIMUM_SIGNAL_BYTES))?;
        Signal::<belief_nexus_wire::transport::NexusBootstrap>::from(Vec::<u8>::from(body))
            .restore()?
    };
    nix::unistd::close(0)?;
    belief_nexus_daemon::server::run(bootstrap)
}
