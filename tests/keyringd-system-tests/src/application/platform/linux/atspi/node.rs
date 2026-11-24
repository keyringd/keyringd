use atspi::CacheItem;

pub(super) struct Node {
    pub(super) item: CacheItem,
    pub(super) sequence: u64,
}
