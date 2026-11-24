use super::TraversalLimit;

use std::collections::HashSet;
use std::hash::Hash;

/// Pruned nodes are not visited. Repeated objects are visited once, and a
/// truncated scan is an error, so it can never establish element absence.
pub fn collect<T, K: Eq + Hash>(
    root: T,
    max_depth: usize,
    max_nodes: usize,
    identity: impl Fn(&T) -> K,
    includes: impl Fn(&T) -> bool,
    children: impl Fn(&T) -> Vec<T>,
) -> Result<Vec<T>, TraversalLimit> {
    let mut stack = vec![(root, 0)];
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    while let Some((node, depth)) = stack.pop() {
        if !includes(&node) || !seen.insert(identity(&node)) {
            continue;
        }
        if depth > max_depth || out.len() >= max_nodes {
            return Err(TraversalLimit);
        }
        // Reverse stack insertion preserves the accessibility tree's row order.
        for child in children(&node).into_iter().rev() {
            stack.push((child, depth + 1));
        }
        out.push(node);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "collect" {
            case "Excludes other windows and preserves order despite cycles" {
                verify "return only scoped elements in traversal order even when the tree contains a cycle" {
                    std::thread::spawn(|| {
                        let nodes = collect(
                            0,
                            5,
                            10,
                            |n| *n,
                            |n| *n != 3,
                            |n| match n {
                                0 => vec![1, 3, 2],
                                1 => vec![0],
                                3 => vec![4],
                                _ => vec![],
                            },
                        )
                        .unwrap();
                        assert_eq!(nodes, vec![0, 1, 2]);
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "collect" {
            case "Limits fail instead of returning an incomplete search" {
                verify "return a traversal limit error if the full search cannot complete" {
                    std::thread::spawn(|| {
                        assert_eq!(
                            collect(0, 1, 10, |n| *n, |_| true, |n| vec![n + 1]),
                            Err(TraversalLimit)
                        );
                        assert_eq!(
                            collect(0, 10, 1, |n| *n, |_| true, |n| vec![n + 1]),
                            Err(TraversalLimit)
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
