#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    use anyhow::Context;
    let mut args = std::env::args_os().skip(1);
    let ready = args.next().context("ready file is required")?;
    let token = args.next().context("broker token is required")?;
    let parent = args.next().context("runner PID is required")?;
    let report = args.next().context("report directory is required")?;
    anyhow::ensure!(args.next().is_none(), "unexpected broker argument");
    keyringd_system_tests::authenticator::windows::security_automation::serve(
        std::path::Path::new(&ready),
        &token.to_string_lossy(),
        parent.to_string_lossy().parse()?,
        std::path::Path::new(&report),
    )
}
#[cfg(not(windows))]
fn main() {
    panic!("Windows Security Automation requires Windows");
}
