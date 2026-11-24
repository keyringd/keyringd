use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

const SSHD_POLL_RETRIES: usize = 50;
const SSHD_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) struct TestSshd {
    process: Child,
    pub port: u16,
    pub dir: PathBuf,
}

impl TestSshd {
    pub fn start(authorized_keys_content: &str) -> Self {
        let dir = create_test_dir();
        let host_key_path = generate_host_key(&dir);
        write_authorized_keys(&dir, authorized_keys_content);
        let port = allocate_port();
        write_sshd_config(&dir, &host_key_path, port);
        let process = start_sshd(&dir);
        wait_for_sshd(port);
        Self { process, port, dir }
    }

    pub async fn run_ssh(&self, socket_path: &Path, remote_command: &str) -> std::process::Output {
        let port = self.port.to_string();
        let sock = socket_path.to_path_buf();
        let cmd = remote_command.to_string();

        tokio::process::Command::new("ssh")
            .env("SSH_AUTH_SOCK", &sock)
            .args(["-v"])
            .args(["-F", "/dev/null"])
            .args(["-p", &port])
            .args(["-o", "StrictHostKeyChecking=no"])
            .args(["-o", "UserKnownHostsFile=/dev/null"])
            .args(["-o", "ConnectTimeout=5"])
            .args(["-o", "BatchMode=yes"])
            .arg("127.0.0.1")
            .arg(&cmd)
            .output()
            .await
            .expect(
                "ssh should be executable because SSH integration tests require the OpenSSH client",
            )
    }
}

impl Drop for TestSshd {
    fn drop(&mut self) {
        if let Ok(log) = std::fs::read_to_string(self.dir.join("sshd.log"))
            && !log.is_empty()
        {
            eprintln!("=== SSHD LOG (port {}) ===\n{}", self.port, log);
        }
        let _ = self.process.kill();
        let _ = self.process.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn create_test_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = PathBuf::from(home).join(format!(".keyringd-test-sshd-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn generate_host_key(dir: &Path) -> PathBuf {
    let path = dir.join("host_key");
    if !path.exists() {
        let status = std::process::Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-f"])
            .arg(&path)
            .args(["-N", "", "-q"])
            .status()
            .expect("ssh-keygen should be executable because SSH integration tests require the OpenSSH client tools");
        assert!(status.success(), "ssh-keygen failed with {status}");
    }
    path
}

fn write_authorized_keys(dir: &Path, content: &str) {
    let auth_dir = dir.join("auth");
    std::fs::create_dir_all(&auth_dir).unwrap();
    let path = auth_dir.join("authorized_keys");
    std::fs::write(&path, content).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

fn allocate_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn write_sshd_config(dir: &Path, host_key_path: &Path, port: u16) {
    let auth_keys = dir.join("auth").join("authorized_keys");
    let config = format!(
        "Port {port}\n\
         ListenAddress 127.0.0.1\n\
         HostKey {}\n\
         PubkeyAuthentication yes\n\
         AuthorizedKeysFile {}\n\
         PasswordAuthentication no\n\
         KbdInteractiveAuthentication no\n\
         UsePAM no\n\
         Subsystem sftp /usr/lib/openssh/sftp-server\n\
         PidFile {}\n",
        host_key_path.display(),
        auth_keys.display(),
        dir.join("sshd.pid").display(),
    );
    std::fs::write(dir.join("sshd_config"), &config).unwrap();
}

fn start_sshd(dir: &Path) -> Child {
    let log_file = std::fs::File::create(dir.join("sshd.log")).unwrap();
    std::process::Command::new("/usr/sbin/sshd")
        .args(["-f", &dir.join("sshd_config").to_string_lossy()])
        .args(["-D", "-e"])
        .stderr(log_file)
        .spawn()
        .expect(
            "sshd should be executable because SSH integration tests require the OpenSSH server",
        )
}

fn wait_for_sshd(port: u16) {
    for _ in 0..SSHD_POLL_RETRIES {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(SSHD_POLL_INTERVAL);
    }
    panic!(
        "sshd did not open port {port} within {:?}",
        SSHD_POLL_INTERVAL * SSHD_POLL_RETRIES as u32
    );
}
