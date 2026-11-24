use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

pub(crate) struct OpenSshInstallation {
    pub(crate) directory: PathBuf,
}

impl OpenSshInstallation {
    pub(crate) fn installed() -> Result<Self> {
        let directory =
            PathBuf::from(std::env::var_os("SystemRoot").context("Windows SystemRoot is not set")?)
                .join("System32/OpenSSH");
        for executable in ["sshd.exe", "ssh.exe", "ssh-keygen.exe"] {
            ensure!(
                directory.join(executable).is_file(),
                "{} is missing; install Windows OpenSSH Client and Server before running UI tests (see tests/keyringd-system-tests/README.md)",
                directory.join(executable).display()
            );
        }
        Ok(Self { directory })
    }
}
