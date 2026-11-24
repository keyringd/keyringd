use std::os::unix::fs::PermissionsExt;

use crate::system::socket::UnixSocketListener;

keyringd_testkit::scenario! {
    case "Creates an owner-only Unix Agent endpoint" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-6"
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-10"
        async: tokio;

        given "a path in a private per-user runtime directory" {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("agent.sock");
        }
        when "the Linux Agent binds its Unix socket" {
            let listener = UnixSocketListener::bind(&path).unwrap();
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        }
        then "only the owning user has read and write permission" {
            assert_eq!(mode, 0o600);
            drop(listener);
        }
    }

    case "Declares an owner-only systemd Agent socket" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-6"
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-10"

        verify "the installed user socket has a per-user runtime path and mode 0600" {
            let unit = include_str!(
                "../../../../../packaging/linux/rootfs/usr/lib/systemd/user/keyringd-agent.socket"
            );
            assert!(unit.lines().any(|line| line == "ListenStream=%t/keyringd-agent.sock"));
            assert!(unit.lines().any(|line| line == "SocketMode=0600"));
        }
    }
}
