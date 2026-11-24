use crate::profile_list_item::ProfileListItemEvent;
#[derive(Clone)]
pub enum ProfileListEvent {
    AddRequested,
    ProfileAction(Box<ProfileListItemEvent>),
}
