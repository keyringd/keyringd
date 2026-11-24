use anyhow::Context;
use tracing_subscriber::EnvFilter;

pub(crate) struct MacOsLogging;

impl MacOsLogging {
    pub(crate) fn init() -> anyhow::Result<()> {
        let home = dirs::home_dir().context("Home directory is unavailable")?;
        let directory = home.join("Library/Logs/keyringd");
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let writer = tracing_appender::rolling::RollingFileAppender::builder()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("keyringd.log")
            .build(&directory)
            .with_context(|| format!("could not create log files in {}", directory.display()))?;
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_env_filter(
                filter.add_directive(
                    "keyringd_context=info"
                        .parse()
                        .expect("valid context log directive"),
                ),
            )
            .with_writer(writer)
            .try_init()
            .map_err(|error| anyhow::anyhow!("could not initialize logging: {error}"))?;
        keyringd_core::util::FailureDiagnostics::install();
        super::fatal_signal_log::FatalSignalLog::install(&directory)
            .context("could not install fatal signal logging")?;
        tracing::info!("keyringd started");
        Ok(())
    }
}
