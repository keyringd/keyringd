use atspi::zbus::zvariant::Type;
use atspi::{CacheItem, InterfaceSet, ObjectRefOwned, Role, StateSet};

#[derive(serde::Deserialize, Type)]
#[zvariant(crate = "atspi::zbus::zvariant")]
pub(super) struct CacheRecord {
    pub(super) object: ObjectRefOwned,
    pub(super) app: ObjectRefOwned,
    pub(super) parent: ObjectRefOwned,
    pub(super) index: i32,
    pub(super) children: i32,
    pub(super) ifaces: InterfaceSet,
    pub(super) short_name: String,
    pub(super) role: u32,
    pub(super) name: String,
    pub(super) states: StateSet,
}

// GTK may expose roles newer than atspi's Role enum. Deserialize the numeric
// role first so one unknown value does not discard the entire bulk snapshot.

impl From<CacheRecord> for CacheItem {
    fn from(record: CacheRecord) -> Self {
        Self {
            object: record.object,
            app: record.app,
            parent: record.parent,
            index: record.index,
            children: record.children,
            ifaces: record.ifaces,
            short_name: record.short_name,
            role: Role::try_from(record.role).unwrap_or(Role::Invalid),
            name: record.name,
            states: record.states,
        }
    }
}
