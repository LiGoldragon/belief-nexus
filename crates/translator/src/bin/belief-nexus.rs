use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let socket = arguments.next().ok_or("TRANSLATOR_SOCKET")?;
    let datom = arguments.next().ok_or("DATOM")?;
    if arguments.next().is_some() { return Err("belief-nexus takes one inline datom".into()); }
    let answer = belief_nexus_translator::text_request(
        Path::new(&socket), &datom, signal::FrameCapacity::default(),
    )?;
    println!("{answer}");
    Ok(())
}
