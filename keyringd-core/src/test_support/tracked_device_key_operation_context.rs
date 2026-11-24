#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TrackedDeviceKeyOperationContext {
    #[default]
    Unspecified,
    SuppliedByCaller,
}
