#[derive(Debug, PartialEq, Eq)]
pub struct TraversalLimit;

impl std::fmt::Display for TraversalLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "accessibility traversal exceeded its depth/node limit; absence cannot be established",
        )
    }
}

impl std::error::Error for TraversalLimit {}
