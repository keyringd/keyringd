use std::collections::BTreeMap;

use super::global_state::GlobalState;
use super::report_bits::ReportBits;
use super::{HidReportDescriptorError, HidReportLayout};

pub fn parse_report_layout(descriptor: &[u8]) -> Result<HidReportLayout, HidReportDescriptorError> {
    let mut reports = BTreeMap::<u8, ReportBits>::new();
    let mut state = GlobalState::default();
    let mut stack = Vec::new();
    let mut offset = 0usize;
    let mut uses_report_ids = false;

    while offset < descriptor.len() {
        let prefix = descriptor[offset];
        offset += 1;
        if prefix == 0xfe {
            let length = *descriptor
                .get(offset)
                .ok_or(HidReportDescriptorError::Truncated)? as usize;
            offset = offset
                .checked_add(2)
                .and_then(|value| value.checked_add(length))
                .filter(|value| *value <= descriptor.len())
                .ok_or(HidReportDescriptorError::Truncated)?;
            continue;
        }

        let length = match prefix & 0x03 {
            3 => 4,
            value => usize::from(value),
        };
        let end = offset
            .checked_add(length)
            .filter(|value| *value <= descriptor.len())
            .ok_or(HidReportDescriptorError::Truncated)?;
        let value = descriptor[offset..end]
            .iter()
            .enumerate()
            .fold(0u64, |result, (index, byte)| {
                result | (u64::from(*byte) << (index * 8))
            });
        offset = end;

        let item_type = (prefix >> 2) & 0x03;
        let tag = prefix >> 4;
        match (item_type, tag) {
            (1, 7) => state.report_size = value,
            (1, 8) => {
                let report_id = u8::try_from(value)
                    .ok()
                    .filter(|value| *value != 0)
                    .ok_or(HidReportDescriptorError::InvalidGlobalState)?;
                state.report_id = report_id;
                uses_report_ids = true;
            }
            (1, 9) => state.report_count = value,
            (1, 10) => stack.push(state),
            (1, 11) => {
                state = stack
                    .pop()
                    .ok_or(HidReportDescriptorError::InvalidGlobalState)?;
            }
            (0, 8 | 9) => {
                let bits = state
                    .report_size
                    .checked_mul(state.report_count)
                    .ok_or(HidReportDescriptorError::ReportTooLarge)?;
                let report = reports.entry(state.report_id).or_default();
                if tag == 8 {
                    report.input = report
                        .input
                        .checked_add(bits)
                        .ok_or(HidReportDescriptorError::ReportTooLarge)?;
                } else {
                    report.output = report
                        .output
                        .checked_add(bits)
                        .ok_or(HidReportDescriptorError::ReportTooLarge)?;
                }
            }
            _ => {}
        }
    }

    let mut candidates = reports.into_iter().filter(|(report_id, bits)| {
        (!uses_report_ids || *report_id != 0) && bits.input != 0 && bits.output != 0
    });
    let (report_id, bits) = candidates
        .next()
        .ok_or(HidReportDescriptorError::MissingOrAmbiguousReport)?;
    if candidates.next().is_some() {
        return Err(HidReportDescriptorError::MissingOrAmbiguousReport);
    }
    if bits.input % 8 != 0 || bits.output % 8 != 0 {
        return Err(HidReportDescriptorError::NonByteAlignedReport);
    }
    let input_packet_size =
        usize::try_from(bits.input / 8).map_err(|_| HidReportDescriptorError::ReportTooLarge)?;
    let output_packet_size =
        usize::try_from(bits.output / 8).map_err(|_| HidReportDescriptorError::ReportTooLarge)?;
    if input_packet_size < 7 || output_packet_size < 7 {
        return Err(HidReportDescriptorError::MissingOrAmbiguousReport);
    }
    Ok(HidReportLayout {
        report_id: uses_report_ids.then_some(report_id),
        input_packet_size,
        output_packet_size,
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    keyringd_testkit::scenario! {
        group "parse_report_layout" {
            case "Reject a cut-off report descriptor" {
                why "The HID descriptor parser must fail closed on truncated variable-length items so malformed device metadata cannot be read past its boundary."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-discovery"
                verify "Reject a cut-off report descriptor" {
                    assert_eq!(
                        parse_report_layout(&[0x95]),
                        Err(HidReportDescriptorError::Truncated)
                    );
                }
            }
        }
    }
}
