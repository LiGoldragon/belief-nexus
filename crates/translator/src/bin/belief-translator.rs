use belief_nexus_translator::{deployment_sets, BeliefTranslator};
use translator_serving::{BootstrapReading, TranslatorBootstrap, TranslatorServing};

/// The translator takes its deployment, the settings its place starts with,
/// as its one argument, and its bootstrap from the pair launcher on standard
/// input. It serves at once and applies the deployment beside; a deployment
/// that cannot be applied ends it, and its pair with it.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let (Some(deployment), None) = (arguments.next(), arguments.next()) else {
        return Err("usage: belief-translator '<deployment>'".into());
    };
    let sets = deployment_sets(&deployment)?;
    let bootstrap = TranslatorBootstrap::read_standard_input()?;
    let translator = BeliefTranslator::try_from(&bootstrap)?;
    let deploying = translator.clone();
    std::thread::spawn(move || {
        if let Err(error) = deploying.deploy(sets) {
            eprintln!("Belief Nexus translator deployment failed: {error}");
            std::process::exit(1);
        }
    });
    translator.serve(&bootstrap)?;
    Ok(())
}
