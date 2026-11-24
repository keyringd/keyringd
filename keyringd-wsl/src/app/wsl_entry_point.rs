use clap::Parser;

use super::CommandLine;
use super::handler::WslProxyHandler;
use crate::system::WindowsBridgeRelay;

pub(crate) struct WslEntryPoint;

impl WslEntryPoint {
    pub(crate) async fn run() -> anyhow::Result<()> {
        tracing_subscriber::fmt::init();
        let command_line = CommandLine::parse();
        WslProxyHandler::run(WindowsBridgeRelay::new(command_line.windows_exe)?).await
    }
}
