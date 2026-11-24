use keyringd_core::model::ProfileId;

/// Moves a profile to a boundary in the original order (before removing it).
pub fn reorder_profile(
    ids: &[ProfileId],
    source: &str,
    insertion: usize,
) -> Option<Vec<ProfileId>> {
    let source = ids.iter().position(|id| id.as_str() == source)?;
    let mut reordered = ids.to_vec();
    let moved = reordered.remove(source);
    let insertion = insertion.saturating_sub(usize::from(source < insertion));
    reordered.insert(insertion.min(reordered.len()), moved);
    Some(reordered)
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "reorder_profile" {
            case "Moving before and after accounts for removed source" {
                verify "compute the insertion index after taking the source row out of the list" {
                    let ids: Vec<_> = ["a", "b", "c"]
                        .map(|s| ProfileId::from_stored(s.into()))
                        .into();
                    let names = |ids: Vec<ProfileId>| {
                        ids.iter()
                            .map(|id| id.as_str().to_owned())
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(
                        names(reorder_profile(&ids, "a", 3).unwrap()),
                        ["b", "c", "a"]
                    );
                    assert_eq!(
                        names(reorder_profile(&ids, "c", 0).unwrap()),
                        ["c", "a", "b"]
                    );
                    assert_eq!(reorder_profile(&ids, "b", 1), Some(ids.clone()));
                    assert_eq!(reorder_profile(&ids, "b", 2), Some(ids.clone()));
                    assert!(reorder_profile(&ids, "missing", 0).is_none());
                }
            }
        }
    }
}
