use std::os::unix::fs::PermissionsExt;

keyringd_testkit::scenario! {
    case "Owns an owner-only macOS agent socket and closes it with the process listener" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-6"
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-10"
        async: tokio;
        timeout: std::time::Duration::from_secs(10);
        verify "bind a private socket, reject a second instance, and close the endpoint on drop" {
            let directory = tempfile::tempdir_in("/tmp").unwrap();
            let path = directory.path().join("keyringd/agent.sock");
            let listener = crate::system::socket::UnixSocketListener::bind(&path).unwrap();
            assert_eq!(std::fs::metadata(path.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
            assert!(std::os::unix::net::UnixStream::connect(&path).is_ok());
            assert_eq!(crate::system::socket::UnixSocketListener::bind(&path).err().unwrap().kind(), std::io::ErrorKind::AddrInUse);
            drop(listener);
            assert!(std::os::unix::net::UnixStream::connect(&path).is_err());
            assert!(!path.exists());
        }
    }
}
