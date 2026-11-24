use super::{Endpoints, io};
use crate::authenticator_server::manager::{
    AuthenticatorServerOptions, AuthenticatorServerProcessError,
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
};

/// Drop is active as soon as Java is spawned, including while startup is pending.
pub(crate) struct OwnedProcess {
    child: Option<Child>,
    log_path: PathBuf,
}
impl OwnedProcess {
    pub fn spawn(
        options: &AuthenticatorServerOptions,
        endpoints: &Endpoints,
    ) -> Result<Self, AuthenticatorServerProcessError> {
        let log = File::create(&options.log_path).map_err(|e| io("create log", e))?;
        let mut command = Command::new(&options.java_executable);
        command
            .args([
                "-Dstdout.encoding=UTF-8",
                "-Dstderr.encoding=UTF-8",
                "-Dorg.slf4j.simpleLogger.defaultLogLevel=warn",
                "-Dorg.slf4j.simpleLogger.log.com.keyringd=info",
                "-Dorg.slf4j.simpleLogger.log.com.webauthn4j.ctap=debug",
                "-jar",
            ])
            .arg(&options.jar)
            .arg(format!("--usb-host={}", endpoints.usb.ip()))
            .arg(format!("--usb-port={}", endpoints.usb.port()))
            .arg(format!("--control-host={}", endpoints.control.ip()))
            .arg(format!("--control-port={}", endpoints.control.port()))
            .stdin(Stdio::null())
            .stdout(Stdio::from(
                log.try_clone().map_err(|e| io("clone log", e))?,
            ))
            .stderr(Stdio::from(log));
        if let Some(hybrid) = &options.hybrid {
            let address = endpoints.hybrid.expect("Hybrid listener reserved");
            command
                .arg(format!("--hybrid-host={}", address.ip()))
                .arg(format!("--hybrid-port={}", address.port()))
                .arg(format!(
                    "--hybrid-certificate-chain={}",
                    hybrid.certificate_chain_pem.display()
                ))
                .arg(format!(
                    "--hybrid-private-key={}",
                    hybrid.private_key_pem.display()
                ));
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let child = command.spawn().map_err(|e| io("launch Java", e))?;
        Ok(Self {
            child: Some(child),
            log_path: options.log_path.clone(),
        })
    }
    pub fn id(&self) -> Option<u32> {
        self.child.as_ref().map(Child::id)
    }
    pub fn status(&mut self) -> Result<Option<ExitStatus>, AuthenticatorServerProcessError> {
        self.child
            .as_mut()
            .expect("startup owns its child")
            .try_wait()
            .map_err(|e| io("poll Java", e))
    }
    pub fn stop(&mut self) -> Result<(), AuthenticatorServerProcessError> {
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        if child
            .try_wait()
            .map_err(|e| io("poll Java before stop", e))?
            .is_none()
            && let Err(error) = child.kill()
        {
            // The child may exit between polling and signalling.
            if child
                .try_wait()
                .map_err(|e| io("poll Java after kill failure", e))?
                .is_none()
            {
                return Err(io("stop Java", error));
            }
        }
        child.wait().map_err(|e| io("reap Java", e))?;
        self.child = None;
        Ok(())
    }
    pub fn log_tail(&self) -> String {
        let contents = (|| -> std::io::Result<String> {
            let mut file = File::open(&self.log_path)?;
            let length = file.metadata()?.len();
            file.seek(SeekFrom::Start(length.saturating_sub(16_384)))?;
            let mut bytes = Vec::new();
            file.take(16_384).read_to_end(&mut bytes)?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        })();
        format!(
            "{}\n{}",
            self.log_path.display(),
            contents.unwrap_or_else(|e| format!("log unavailable: {e}"))
        )
    }
}
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("failed to clean up owned AuthenticatorServer: {error}");
        }
    }
}
