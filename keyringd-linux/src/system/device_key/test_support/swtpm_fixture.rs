use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use crate::system::device_key::EsapiDeviceKeyService;

const SWTPM_SERVER_PORT: u16 = 2321;
const SWTPM_CTRL_PORT: u16 = 2322;
const SWTPM_POLL_RETRIES: usize = 50;
const SWTPM_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub static TPM_LOCK: Mutex<()> = Mutex::new(());

static SWTPM: LazyLock<SwtpmFixture> = LazyLock::new(start_swtpm);

struct SwtpmFixture {
    _child: Option<Child>,
    _state: Option<tempfile::TempDir>,
}

unsafe impl Sync for SwtpmFixture {}

pub fn ensure_swtpm() {
    LazyLock::force(&SWTPM);
}

fn start_swtpm() -> SwtpmFixture {
    if swtpm_available() {
        return SwtpmFixture {
            _child: None,
            _state: None,
        };
    }

    let state = tempfile::tempdir().unwrap();
    let setup = Command::new("swtpm_setup")
        .args(["--tpm2", "--tpm-state"])
        .arg(state.path())
        .arg("--overwrite")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("swtpm_setup should be executable because TPM tests require swtpm-tools to be installed");
    assert!(
        setup.status.success(),
        "swtpm_setup failed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let mut child = Command::new("swtpm")
        .args(["socket", "--tpm2", "--flags", "not-need-init", "--tpmstate"])
        .arg(format!("dir={}", state.path().display()))
        .args([
            "--server",
            &format!("type=tcp,port={SWTPM_SERVER_PORT}"),
            "--ctrl",
            &format!("type=tcp,port={SWTPM_CTRL_PORT}"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("swtpm should be executable because TPM tests require swtpm to be installed");

    for _ in 0..SWTPM_POLL_RETRIES {
        if swtpm_available() {
            return SwtpmFixture {
                _child: Some(child),
                _state: Some(state),
            };
        }
        if child.try_wait().unwrap().is_some() {
            panic!("swtpm exited before opening port {SWTPM_SERVER_PORT}");
        }
        std::thread::sleep(SWTPM_POLL_INTERVAL);
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("swtpm did not open port {SWTPM_SERVER_PORT}");
}

fn swtpm_available() -> bool {
    TcpStream::connect(format!("127.0.0.1:{SWTPM_SERVER_PORT}")).is_ok()
}

pub fn require_swtpm() -> EsapiDeviceKeyService {
    ensure_swtpm();
    EsapiDeviceKeyService::new(tss_esapi::TctiNameConf::Swtpm(Default::default()), None)
}

pub fn sha2_digest(data: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(data).to_vec()
}
