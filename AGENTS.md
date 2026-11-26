# Engineering guidelines

## Tests

- Name Cargo features that enable test-only code `test-support` in every crate.
- Add a regression test when the same bug could realistically be reintroduced.
- When adapting to a one-time external-tool change, verify the integration by running that tool.
- For test placement, naming, and system-test responsibilities, follow `tests/README.md`.
- For supported build and test commands, follow `xtask/README.md`.
- Keep specification tests under `src/tests/spec/<specification>/`.

## Rust source layout

- Use one primary type per file by default. Name the file after that type in snake_case.
- Keep a type, its implementations, dedicated helpers, and unit tests in the same file.
- Types shared independently of a single parent belong in separate files.
- Name directories by responsibility, not merely after a type.
- Keep `mod.rs` limited to child declarations and necessary re-exports.
- Prefer private modules and the narrowest useful visibility.
- Avoid `include!` and `#[path]` as substitutes for ordinary module organization.

## Pull requests

- Write PR titles and descriptions in English.
- Keep PR descriptions concise and limited to essential information. Do not divide them into sections unless explicitly instructed.
- Explain what changed, why it changed, and anything reviewers need to know.
- Do not list local results for checks already run by CI.
- Mention verification outside CI when it materially helps review.
