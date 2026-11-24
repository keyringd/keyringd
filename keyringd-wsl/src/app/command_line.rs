use clap::Parser;

#[derive(Parser)]
#[command(
    name = "keyringd-wsl-proxy",
    about = "Expose the Windows keyringd agent to WSL"
)]
pub(crate) struct CommandLine {
    #[arg(long)]
    pub(crate) windows_exe: Option<std::path::PathBuf>,
}
