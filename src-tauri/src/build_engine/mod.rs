pub mod bitset_miner;
pub mod cache;
pub mod data_loader;
pub mod host;
pub mod ipc;
pub mod item_catalog;
pub mod models;

pub use models::*;

pub fn run_build_engine_process() {
    if let Err(e) = ipc::run_ipc_stdin_stdout() {
        eprintln!("lcu-build-engine error: {}", e);
    }
}
