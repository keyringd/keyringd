use std::path::PathBuf;

pub(crate) enum StartupMode {
    Agent {
        socket: PathBuf,
        data_dir: PathBuf,
        accept_remote_hybrid_advertisements: bool,
    },
    Settings,
    RelayHybridAdvertisements {
        agent_socket: PathBuf,
    },
}
