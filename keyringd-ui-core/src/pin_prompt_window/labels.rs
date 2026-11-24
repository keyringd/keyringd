use crate::text;
pub fn pin_heading(new_pin: bool) -> &'static str {
    if new_pin {
        text::PIN_PROMPT_CREATE_HEADING
    } else {
        text::PIN_PROMPT_ENTER_HEADING
    }
}

pub fn new_pin_reason(minimum_length: u64, rejected: bool) -> String {
    if rejected {
        format!(
            "The security key rejected that PIN. Choose a different PIN with at least {minimum_length} characters."
        )
    } else {
        format!("Choose a PIN with at least {minimum_length} characters for this security key.")
    }
}
pub fn pin_explanation(
    reason: &str,
    minimum_length: Option<u32>,
    attempts_left: Option<u32>,
) -> String {
    let mut explanation = reason.to_owned();
    if let Some(minimum) = minimum_length {
        explanation = format!(
            "{}. Use at least {minimum} characters.",
            explanation.trim_end_matches('.')
        );
    }
    if let Some(attempts) = attempts_left {
        explanation.push_str(&format!(" {attempts} attempts remaining."));
    }
    explanation
}
