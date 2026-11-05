fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let metaflow = arguments.next().ok_or("METAFLOW")?;
    let datom = arguments.next().ok_or("DATOM")?;
    let socket_name = arguments.next().ok_or("META_TRANSLATOR_SOCKET_NAME")?;
    if arguments.next().is_some() { return Err("unexpected internal argument".into()); }
    let runtime = shared_translator::placement::resolve_runtime(&metaflow)?;
    let socket = std::path::PathBuf::from(runtime.0).join(socket_name);
    let answer = belief_nexus_translator::text_request(
        &socket, &datom, signal::FrameCapacity::default(),
    )?;
    println!("{answer}");
    Ok(())
}
