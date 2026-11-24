//! Bounded traversal shared with the AppKit fixture; testable without AppKit.
mod traversal_limit;
pub use traversal_limit::TraversalLimit;
mod traversal;
pub use traversal::collect;
