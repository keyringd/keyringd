#[derive(Clone, Debug)]
pub(crate) enum DistTarget {
    Linux,
    Wsl,
    Windows,
    Macos,
}
