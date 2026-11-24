use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use atspi::events::object::Property;
use atspi::events::{CacheEvents, Event, ObjectEvents};
use atspi::{CacheItem, ObjectRefOwned, Operation};

use super::Node;
use super::Traversal;

#[derive(Default)]
pub(super) struct TreeState {
    nodes: HashMap<ObjectRefOwned, Node>,
    sequence: u64,
    dirty: bool,
}

impl TreeState {
    pub(super) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(super) fn contains(&self, object: &ObjectRefOwned) -> bool {
        self.nodes.contains_key(object)
    }

    pub(super) fn insert(&mut self, item: CacheItem) {
        self.sequence += 1;
        self.nodes.insert(
            item.object.clone(),
            Node {
                item,
                sequence: self.sequence,
            },
        );
    }

    pub(super) fn merge_window(&mut self, items: Vec<CacheItem>) {
        for item in items {
            self.insert(item);
        }
    }

    pub(super) fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub(super) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(super) fn reachable(
        &self,
        roots: Vec<ObjectRefOwned>,
        traversal: Traversal,
    ) -> Option<Vec<CacheItem>> {
        if roots.iter().any(|root| !self.contains(root)) {
            return None;
        }
        let mut children: HashMap<&ObjectRefOwned, BTreeMap<i32, (u64, &ObjectRefOwned)>> =
            HashMap::new();
        for (object, node) in &self.nodes {
            if node.item.index < 0 {
                continue;
            }
            let siblings = children.entry(&node.item.parent).or_default();
            let slot = siblings.entry(node.item.index).or_insert((0, object));
            if node.sequence > slot.0 {
                *slot = (node.sequence, object);
            }
        }
        let mut pending = VecDeque::from(roots);
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        while let Some(object) = match traversal {
            Traversal::DepthFirst => pending.pop_back(),
            Traversal::BreadthFirst => pending.pop_front(),
        } {
            if !seen.insert(object.clone()) {
                continue;
            }
            let Some(node) = self.nodes.get(&object) else {
                continue;
            };
            result.push(node.item.clone());
            if let Some(siblings) = children.get(&object) {
                pending.extend(siblings.values().map(|(_, child)| (*child).clone()));
            }
        }
        Some(result)
    }

    pub(super) fn apply(&mut self, event: Event) {
        match event {
            Event::Cache(CacheEvents::Add(event)) => self.insert(event.node_added),
            Event::Cache(CacheEvents::Remove(event)) => {
                self.nodes.remove(&event.node_removed);
            }
            Event::Object(ObjectEvents::ChildrenChanged(event)) => match event.operation {
                Operation::Insert => self.mark_dirty(),
                Operation::Delete => {
                    self.nodes.remove(&event.child);
                }
            },
            Event::Object(ObjectEvents::PropertyChange(event)) => {
                if let Some(node) = self.nodes.get_mut(&event.item) {
                    match event.value {
                        Property::Name(name) => node.item.short_name = name,
                        Property::Role(role) => node.item.role = role,
                        Property::Parent(parent) => {
                            node.item.parent = parent;
                            self.mark_dirty();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
