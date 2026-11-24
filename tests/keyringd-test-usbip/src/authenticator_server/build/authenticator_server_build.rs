use anyhow::{Context, Result, ensure};
use std::{fs, path::PathBuf, process::Command};

/// Builds the workspace's Kotlin server and returns its executable JAR.
/// Gradle owns incremental builds and locking of the shared project cache.
pub struct AuthenticatorServerBuild {
    workspace_root: PathBuf,
    java_home: Option<PathBuf>,
}

impl AuthenticatorServerBuild {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            java_home: None,
        }
    }

    /// Select a JDK explicitly; otherwise Gradle uses JAVA_HOME or PATH.
    pub fn with_java_home(mut self, java_home: impl Into<PathBuf>) -> Self {
        self.java_home = Some(java_home.into());
        self
    }

    /// Requires a JDK and the repository's Gradle wrapper. This blocking API
    /// belongs in environment setup, before starting a server.
    pub fn prepare(&self) -> Result<PathBuf> {
        let root = fs::canonicalize(&self.workspace_root)
            .context("resolve AuthenticatorServer workspace root")?;
        #[cfg(windows)]
        let root = super::batch_path::batch_path(&root);
        let project = root.join("tests/keyringd-test-usbip/authenticator-server");
        let wrapper = project.join(if cfg!(windows) {
            "gradlew.bat"
        } else {
            "gradlew"
        });
        ensure!(
            wrapper.is_file() && project.join("build.gradle.kts").is_file(),
            "AuthenticatorServer Gradle project is missing: {}",
            project.display()
        );
        let output = root.join("target/authenticator-server");
        fs::create_dir_all(&output).context("create AuthenticatorServer build directory")?;
        let mut command = Command::new(wrapper);
        command
            .current_dir(&project)
            .arg("--no-daemon")
            .arg("--project-cache-dir")
            .arg(output.join("gradle-project-cache"))
            .arg("buildFatJar");
        if let Some(java_home) = &self.java_home {
            #[cfg(windows)]
            let java_home = super::batch_path::batch_path(java_home);
            command.env("JAVA_HOME", java_home);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let status = command
            .status()
            .context("run AuthenticatorServer Gradle build (JDK required)")?;
        ensure!(
            status.success(),
            "AuthenticatorServer Gradle build failed: {status}"
        );
        let jar = output.join("libs/keyringd-authenticator-server.jar");
        ensure!(
            jar.is_file(),
            "AuthenticatorServer fat JAR was not produced: {}",
            jar.display()
        );
        Ok(jar)
    }
}
