#![recursion_limit = "256"]

mod app;
mod system;

#[cfg(test)]
mod tests;

pub fn run() -> anyhow::Result<()> {
    app::LinuxEntryPoint::run()
}
