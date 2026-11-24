use keyringd_core::model::ProfileId;

#[derive(Clone)]
pub struct RemoveProfileProps {
    pub profile_id: ProfileId,
    pub profile_name: String,
}
