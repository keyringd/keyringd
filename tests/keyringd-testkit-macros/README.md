# keyringd-testkit-macros

[Proc-Macro Crate which deploys the Scenario and Evidence Macro of TestKit](../keyringd-testkit/README.md) to Rust Test. The user uses Macro, which has been republished from TestKit.

| Module | Duty |
| --- | --- |
| `syntax` | DSL syntax parsing |
| `validation` | Semantic rules such as required elements, step order, name, etc. |
| `expansion` | Deployed to Calls Test Functions, Case Metadata, Execution/Trail API |

Async, Timeout, Examples, Groups, and Skip/Ignored handling are [TestKit DSL Description](../keyringd-testkit/README.md). Run-time TestKit is responsible for saving the results and generating HTML.

```sh
cargo test -p keyringd-testkit-macros
```

Check the analysis, validate, and deploy in Unit Test, and check the compilation of the incorrect syntax in TryBuild. When DSL changes, the descriptions of both and the user side will be updated.
