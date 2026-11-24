/// Failed stages retain every original error. `source()` exposes the first;
/// `failures()` also provides the other causes, which do not form a linear chain.
#[derive(Debug)]
pub struct StageFailures {
    failures: Vec<(String, anyhow::Error)>,
}

impl StageFailures {
    pub fn finish<S: Into<String>>(
        stages: impl IntoIterator<Item = (S, anyhow::Result<()>)>,
    ) -> anyhow::Result<()> {
        let failures = stages
            .into_iter()
            .filter_map(|(name, result)| result.err().map(|error| (name.into(), error)))
            .collect::<Vec<_>>();
        if failures.is_empty() {
            Ok(())
        } else {
            Err(Self { failures }.into())
        }
    }

    pub fn failures(&self) -> &[(String, anyhow::Error)] {
        &self.failures
    }
}

impl std::fmt::Display for StageFailures {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, (name, error)) in self.failures.iter().enumerate() {
            if index > 0 {
                writeln!(formatter)?;
            }
            write!(formatter, "{name}: {error:#}")?;
        }
        Ok(())
    }
}

impl std::error::Error for StageFailures {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.failures.first().map(|(_, error)| error.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    keyringd_testkit::scenario! {
        group "StageFailures::finish" {
            case "Retains all original error types and the first cause" {
                verify "preserve each stage's typed error and expose the first through the error source" {
                    std::thread::spawn(|| {
                        let first = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
                        let second = "not a number".parse::<u32>().unwrap_err();
                        let error =
                            StageFailures::finish([("build", Err(first.into())), ("report", Err(second.into()))])
                                .unwrap_err();
                        let failures = error.downcast_ref::<StageFailures>().unwrap();
                        assert_eq!(failures.failures().len(), 2);
                        assert_eq!(failures.failures()[0].0, "build");
                        assert_eq!(
                            failures
                                .source()
                                .unwrap()
                                .downcast_ref::<std::io::Error>()
                                .unwrap()
                                .kind(),
                            std::io::ErrorKind::PermissionDenied
                        );
                        assert!(failures.failures()[1].1.is::<std::num::ParseIntError>());
                        assert!(error.chain().any(|cause| cause.is::<std::io::Error>()));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "StageFailures::finish" {
            case "Successful stages are not errors" {
                verify "return no aggregate error when every recorded stage succeeds" {
                    std::thread::spawn(|| {
                        assert!(StageFailures::finish([("build", Ok(())), ("report", Ok(()))]).is_ok());
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
