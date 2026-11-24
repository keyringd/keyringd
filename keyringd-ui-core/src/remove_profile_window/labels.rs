pub fn heading(name: &str) -> String {
    format!("Remove “{name}”?")
}

pub fn removed_notice(name: &str) -> String {
    format!("Removed “{name}”.")
}

pub fn failed_notice(name: &str, detail: &str) -> String {
    format!("Could not remove “{name}”. {detail}")
}
