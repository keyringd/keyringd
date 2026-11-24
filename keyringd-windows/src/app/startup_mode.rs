pub(crate) enum StartupMode {
    Desktop,
    StdioBridge,
}

impl StartupMode {
    pub(crate) fn parse() -> anyhow::Result<Self> {
        match std::env::args_os().nth(1).as_deref() {
            None => Ok(Self::Desktop),
            Some(value) if value == std::ffi::OsStr::new("bridge") => Ok(Self::StdioBridge),
            Some(value) => anyhow::bail!("unknown command: {}", value.to_string_lossy()),
        }
    }
}
