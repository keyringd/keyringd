mod app;
mod core;

fn main() -> anyhow::Result<()> {
    if let Some(mut compiler) = core::service::coverage::compiler_command()? {
        std::process::exit(compiler.status()?.code().unwrap_or(1));
    }
    app::run()
}
