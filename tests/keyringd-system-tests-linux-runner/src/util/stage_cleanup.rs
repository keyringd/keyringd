pub(crate) fn finish<const N: usize>(
    stages: [(&str, anyhow::Result<()>); N],
) -> anyhow::Result<()> {
    keyringd_testkit::runner::StageFailures::finish(stages)
}
