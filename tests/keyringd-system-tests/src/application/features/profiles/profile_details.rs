/// Values observed in the product Profile Details window.
#[derive(Debug, PartialEq, Eq)]
pub struct ProfileDetails {
    pub status: String,
    pub signature_algorithm: String,
    pub protection: String,
}
