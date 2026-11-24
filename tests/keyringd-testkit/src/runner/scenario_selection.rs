use anyhow::{Result, ensure};
use std::collections::BTreeSet;

/// The nonempty, unique set of tests selected by libtest's `--list` output.
#[derive(Debug)]
pub struct ScenarioSelection {
    cases: BTreeSet<String>,
}
impl ScenarioSelection {
    pub fn from_listing(listing: &str) -> Result<Self> {
        let mut cases = BTreeSet::new();
        for case in listing
            .lines()
            .filter_map(|line| line.strip_suffix(": test"))
        {
            ensure!(!case.is_empty(), "empty listed case name");
            ensure!(
                cases.insert(case.to_owned()),
                "duplicate listed case: {case}"
            );
        }
        ensure!(!cases.is_empty(), "no scenario cases matched the selection");
        Ok(Self { cases })
    }
    pub fn cases(&self) -> &BTreeSet<String> {
        &self.cases
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "ScenarioSelection::from_listing" {
            case "Preserves unicode names and ignores benchmarks and summary" {
                verify "preserve Unicode test names when filtering the libtest listing" {
                    std::thread::spawn(|| {
                    let selected = ScenarioSelection::from_listing("module::creates_parent_key: test\nmodule::other: test\nbenchmark: benchmark\n2 tests, 1 benchmark\n").unwrap();
                            assert_eq!(selected.cases().len(), 2);
                            assert!(selected.cases().contains("module::creates_parent_key"));
                    }).join().unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "ScenarioSelection::from_listing" {
            case "Rejects empty or duplicate selections" {
                verify "reject listings with no selected tests or repeated names" {
                    std::thread::spawn(|| {
                        for listing in ["0 tests", ": test", "case: test\ncase: test"] {
                            assert!(ScenarioSelection::from_listing(listing).is_err());
                        }
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
