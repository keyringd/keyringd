use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result};
use atspi::events::{CacheEvents, Event, ObjectEvents};
use atspi::proxy::accessible::ObjectRefExt;
use atspi::{AccessibilityConnection, CacheItem, ObjectRefOwned, Role};
use futures_util::StreamExt;
use futures_util::future::join_all;
use tokio::sync::{Mutex, RwLock};

use super::ATSPI_RESPONSE_TIMEOUT;
use super::TreeState;

use super::Traversal;

use super::CacheRecord;

fn event_object(event: &Event) -> Option<&ObjectRefOwned> {
    match event {
        Event::Cache(CacheEvents::Add(add)) => Some(&add.node_added.object),
        Event::Cache(CacheEvents::Remove(remove)) => Some(&remove.node_removed),
        Event::Object(ObjectEvents::ChildrenChanged(change)) => Some(&change.item),
        Event::Object(ObjectEvents::PropertyChange(change)) => Some(&change.item),
        _ => None,
    }
}

pub(super) struct ApplicationTree {
    state: Arc<RwLock<TreeState>>,
    refresh: Mutex<()>,
    root: ObjectRefOwned,
    connection: AccessibilityConnection,
    task: tokio::task::JoinHandle<()>,
}

impl ApplicationTree {
    async fn read_subtree(
        &self,
        root: ObjectRefOwned,
        parent: ObjectRefOwned,
        index: i32,
    ) -> Result<Vec<CacheItem>> {
        let mut pending = vec![(root, parent, index)];
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        while !pending.is_empty() {
            let level = std::mem::take(&mut pending)
                .into_iter()
                .filter(|(object, _, _)| seen.insert(object.clone()))
                .collect::<Vec<_>>();
            let connection = self.connection.connection();
            let fetched = join_all(level.into_iter().map(|(object, parent, index)| async move {
                let accessible = object.as_accessible_proxy(connection).await?;
                let (name, role, children) = tokio::join!(
                    accessible.name(),
                    accessible.get_role(),
                    accessible.get_children(),
                );
                let children = children?;
                let item = CacheItem {
                    object,
                    parent,
                    index,
                    children: children.len() as i32,
                    short_name: name?,
                    role: role.unwrap_or(Role::Invalid),
                    ..CacheItem::default()
                };
                Ok::<_, anyhow::Error>((item, children))
            }))
            .await;
            for node in fetched {
                let (item, children) = node?;
                pending.extend(
                    children
                        .into_iter()
                        .enumerate()
                        .map(|(index, child)| (child, item.object.clone(), index as i32)),
                );
                result.push(item);
            }
            anyhow::ensure!(
                result.len() <= 1_000,
                "AT-SPI window has more than 1,000 nodes"
            );
        }
        Ok(result)
    }

    pub(super) async fn start(
        connection: &AccessibilityConnection,
        application_element: ObjectRefOwned,
    ) -> Result<Self> {
        let bus_name = application_element
            .name()
            .context("application element has no bus name")?
            .clone();
        // The connection already subscribes to cache and object changes. Create
        // the receiver before GetItems so changes during the snapshot are queued.
        let mut events = Box::pin(connection.event_stream());

        let cache = atspi::zbus::Proxy::new(
            connection.connection(),
            bus_name.clone(),
            "/org/a11y/atspi/cache",
            "org.a11y.atspi.Cache",
        )
        .await?;
        let records: Vec<CacheRecord> =
            tokio::time::timeout(ATSPI_RESPONSE_TIMEOUT, cache.call("GetItems", &()))
                .await
                .context("timed out loading the application AT-SPI cache")?
                .context("failed to load the application AT-SPI cache")?;
        let items = records.into_iter().map(CacheItem::from).collect::<Vec<_>>();
        let root = items
            .iter()
            .find(|item| item.object.path_as_str() == "/org/a11y/atspi/accessible/root")
            .map(|item| item.object.clone())
            .context("application AT-SPI cache has no root object")?;
        let mut initial = TreeState::default();
        for item in items {
            initial.insert(item);
        }
        tracing::trace!(nodes = initial.len(), "loaded application AT-SPI tree");
        let state = Arc::new(RwLock::new(initial));
        let task_state = state.clone();
        let task_bus_name = bus_name.to_string();
        let task = tokio::spawn(async move {
            while let Some(event) = events.next().await {
                match event {
                    Ok(event) => {
                        // Cache events from other applications use the same bus.
                        if event_object(&event)
                            .is_some_and(|object| object.name_as_str() == Some(&task_bus_name))
                        {
                            task_state.write().await.apply(event);
                        }
                    }
                    Err(error) => {
                        // atspi currently has no variant for Cache::Ready. Its
                        // decoder reports that signal as an error even though
                        // the connection and our cached tree are still valid.
                        tracing::trace!(%error, "ignored undecodable AT-SPI signal");
                    }
                }
            }
        });
        Ok(Self {
            state,
            refresh: Mutex::new(()),
            root,
            connection: connection.clone(),
            task,
        })
    }

    async fn top_level_children(&self) -> Result<Vec<ObjectRefOwned>> {
        let accessible = self
            .root
            .as_accessible_proxy(self.connection.connection())
            .await?;
        tokio::time::timeout(ATSPI_RESPONSE_TIMEOUT, accessible.get_children())
            .await
            .context("timed out listing application windows")?
            .context("failed to list application windows")
    }

    async fn hydrate_missing(&self, roots: &[ObjectRefOwned]) -> Result<()> {
        let mut state = self.state.write().await;
        let refresh = state.take_dirty() || roots.iter().any(|root| !state.contains(root));
        drop(state);
        if !refresh {
            return Ok(());
        }
        // GTK can omit descendants from Cache::Add even when the window is
        // cached, so rebuild live windows after structural changes.
        for (index, root) in roots.iter().enumerate() {
            let subtree = tokio::time::timeout(
                ATSPI_RESPONSE_TIMEOUT,
                self.read_subtree(root.clone(), self.root.clone(), index as i32),
            )
            .await
            .context("timed out refreshing AT-SPI window")
            .and_then(|result| result);
            let subtree = match subtree {
                Ok(subtree) => subtree,
                Err(error) => {
                    self.state.write().await.mark_dirty();
                    return Err(error);
                }
            };
            tracing::trace!(nodes = subtree.len(), "refreshed AT-SPI window");
            self.state.write().await.merge_window(subtree);
        }
        Ok(())
    }

    pub(super) async fn invalidate(&self) {
        self.state.write().await.mark_dirty();
    }

    pub(super) async fn reachable(&self, traversal: Traversal) -> Option<Vec<CacheItem>> {
        if self.task.is_finished() {
            return None;
        }
        let _refresh = self.refresh.lock().await;
        let roots = match self.top_level_children().await {
            Ok(roots) => roots,
            Err(error) => {
                tracing::trace!(%error, "could not list application windows; using live traversal");
                return None;
            }
        };
        if let Err(error) = self.hydrate_missing(&roots).await {
            tracing::trace!(%error, "could not hydrate AT-SPI branch; using live traversal");
            return None;
        }
        self.state.read().await.reachable(roots, traversal)
    }
}

impl Drop for ApplicationTree {
    fn drop(&mut self) {
        self.task.abort();
    }
}
