#[derive(Clone, Debug)]
pub(crate) enum DistFormat {
    Deb,
    Rpm,
    Msi,
    Runtime,
    /// Signed and notarized macOS app archive, or an ad-hoc development bundle.
    App,
}
