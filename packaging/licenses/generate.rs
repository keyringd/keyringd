use anyhow::{Context, Result};
use cargo_about::licenses::{self, config::Config, resolution};
use krates::Utf8PathBuf;
use std::{collections::BTreeMap, fmt::Write, sync::Arc};
use toml_span::Deserialize;

pub fn generate() -> Result<()> {
    let manifest_dir = Utf8PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let root = manifest_dir.parent().context("missing workspace root")?;
    println!(
        "cargo:rerun-if-changed={}",
        root.join("packaging/licenses/generate.rs")
    );
    let config_path = root.join("packaging/licenses/about.toml");
    for path in [
        root.join("Cargo.toml"),
        root.join("Cargo.lock"),
        config_path.clone(),
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    let config_text = std::fs::read_to_string(&config_path)?;
    let mut config_value = toml_span::parse(&config_text)?;
    let mut config = Config::deserialize(&mut config_value)?;
    // Vendored notices are relative to this configuration, not the Cargo registry.
    for crate_config in config.crates.values_mut() {
        if let Some(clarification) = &mut crate_config.value.clarify {
            for file in &mut clarification.files {
                if file.path.starts_with("vendored") {
                    file.path = config_path.parent().unwrap().join(&file.path);
                }
            }
        }
    }
    let target = std::env::var("TARGET")?;
    // Filter metadata as well as the license graph to the binary's target.
    let mut command = krates::Cmd::new();
    command.manifest_path(manifest_dir.join("Cargo.toml"));
    let manifest_text = std::fs::read_to_string(manifest_dir.join("Cargo.toml"))?;
    let manifest = toml_span::parse(&manifest_text)?;
    command.no_default_features();
    if let Some(features) = manifest
        .pointer("/features")
        .and_then(|value| value.as_table())
    {
        command.features(features.keys().filter_map(|feature| {
            let variable = format!(
                "CARGO_FEATURE_{}",
                feature.name.to_uppercase().replace('-', "_")
            );
            std::env::var_os(variable).map(|_| feature.name.to_string())
        }));
    }
    command.lock_opts(krates::LockOptions {
        locked: true,
        offline: false,
        frozen: false,
    });
    command.other_options(["--filter-platform".to_owned(), target.clone()]);
    let mut builder = krates::Builder::new();
    builder.include_targets([(target, Vec::new())]);
    if config.ignore_build_dependencies {
        builder.ignore_kind(krates::DepKind::Build, krates::Scope::All);
    }
    if config.ignore_dev_dependencies {
        builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
    }
    let crates: cargo_about::Krates = builder.build(command, |_| {})?;
    for krate in crates.krates() {
        println!("cargo:rerun-if-changed={}", krate.manifest_path);
    }
    let store = Arc::new(licenses::store_from_cache()?);
    let mut summary = licenses::Gatherer::with_store(store)
        .with_max_depth(config.max_depth.map(|depth| depth as usize))
        .gather(&crates, &config, None);
    // Workspace code is covered by the application's own Apache-2.0 license.
    summary.retain(|entry| !entry.krate.manifest_path.starts_with(root));
    for entry in &summary {
        for file in &entry.license_files {
            let path = entry.krate.manifest_path.parent().unwrap().join(&file.path);
            println!("cargo:rerun-if-changed={path}");
        }
    }
    let crate_config: BTreeMap<_, _> = config
        .crates
        .into_iter()
        .map(|(name, config)| (name, config.value))
        .collect();
    let mut files = resolution::Files::new();
    let resolved = resolution::resolve(&summary, &config.accepted, &crate_config, &mut files, true);
    let list = cargo_about::generate::generate(&summary, &resolved, |diagnostics| {
        for diagnostic in diagnostics {
            println!("cargo:warning={}", diagnostic.message);
            for note in &diagnostic.notes {
                println!("cargo:warning={note}");
            }
        }
    })?;
    let missing_text: Vec<_> = list
        .licenses
        .iter()
        .filter(|license| license.source_path.is_none())
        .flat_map(|license| {
            license.used_by.iter().map(move |usage| {
                format!(
                    "{} {} ({})",
                    usage.krate.name, usage.krate.version, license.id
                )
            })
        })
        .collect();
    anyhow::ensure!(
        missing_text.is_empty(),
        "license text must come from the dependency, not a generic SPDX template; add a clarification for: {}",
        missing_text.join(", ")
    );
    let mut text = String::from(
        "Third-party Licenses\n====================\n\nRust libraries used by keyringd.\n",
    );
    for license in list.licenses {
        writeln!(
            text,
            "\n--------------------------------------------------------------------------------\n{} ({})\n\nLibraries:",
            license.name, license.id
        )?;
        for usage in license.used_by {
            write!(text, "  {} {}", usage.krate.name, usage.krate.version)?;
            if let Some(repository) = &usage.krate.repository {
                write!(text, " — {repository}")?;
            }
            writeln!(text)?;
        }
        writeln!(text, "\n{}", license.text)?;
    }
    let output = Utf8PathBuf::from(std::env::var("OUT_DIR")?).join("third-party-licenses.txt");
    std::fs::write(output, text).context("failed to write third-party licenses")
}
