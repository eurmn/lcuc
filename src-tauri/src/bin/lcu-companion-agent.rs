fn main() {
    if let Err(error) = lcu_companion_lib::agent::run_process_from_main() {
        lcu_companion_lib::logger::error("agent.process", &format!("Fatal error: {error}"));
        std::process::exit(1);
    }
}
