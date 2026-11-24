use std::os::unix::fs::PermissionsExt;

use crate::system::socket::UnixSocketListener;

keyringd_testkit::scenario! {
    case "Creates an owner-only WSL Agent endpoint" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-6"
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-10"
        async: tokio;

        given "a path in an isolated runtime directory" {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("agent.sock");
        }
        when "the WSL proxy binds its Unix socket" {
            let listener = UnixSocketListener::bind(&path).unwrap();
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        }
        then "only the owning user has read and write permission" {
            assert_eq!(mode, 0o600);
            drop(listener);
        }
    }

    case "Publishes a private discoverable WSL Agent endpoint through systemd" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-6"
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-10"

        verify "the user socket has a stable runtime path and owner-only mode" {
            let unit = include_str!(
                "../../../../../packaging/wsl/rootfs/usr/lib/systemd/user/keyringd.socket"
            );
            assert!(unit.lines().any(|line| line == "ListenStream=%t/keyringd-agent.sock"));
            assert!(unit.lines().any(|line| line == "SocketMode=0600"));
        }
    }
}
