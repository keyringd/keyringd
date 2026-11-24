#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileSetupProgress {
    Starting,
    CreatingPasskey,
    DiscoveringPasskey,
    RecoveringPasskeyPublicKey,
    ProvisioningDeviceKey,
    CommittingProfile,
}
