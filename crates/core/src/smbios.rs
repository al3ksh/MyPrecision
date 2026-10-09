//! Device identity from the raw SMBIOS table (`GetSystemFirmwareTable('RSMB')`).

use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    /// Dell's service tag is the SMBIOS system serial number.
    pub service_tag: Option<String>,
    pub bios_version: Option<String>,
    pub bios_date: Option<String>,
}

/// Parses the buffer `GetSystemFirmwareTable('RSMB')` returns: an 8-byte header
/// (`RawSMBIOSData`) followed by the structure table. Malformed input yields what was readable.
pub fn parse(raw: &[u8]) -> DeviceInfo {
    let mut info = DeviceInfo::default();
    let Some(len) = raw.get(4..8).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize) else {
        return info;
    };
    let table = &raw[8..raw.len().min(8 + len)];

    let mut pos = 0;
    while pos + 4 <= table.len() {
        let kind = table[pos];
        let formatted_len = table[pos + 1] as usize;
        if formatted_len < 4 || pos + formatted_len > table.len() {
            break;
        }
        let formatted = &table[pos..pos + formatted_len];
        let strings_start = pos + formatted_len;
        let strings_end = find_double_nul(table, strings_start).unwrap_or(table.len());
        let strings: Vec<&[u8]> = table[strings_start..strings_end].split(|&b| b == 0).collect();
        let s = |offset: usize| string_at(formatted, &strings, offset);

        match kind {
            0 => {
                info.bios_version = s(0x05);
                info.bios_date = s(0x08);
            }
            1 => {
                info.manufacturer = s(0x04);
                info.model = s(0x05);
                info.service_tag = s(0x07);
            }
            127 => break,
            _ => {}
        }
        pos = strings_end + 2;
    }
    info
}

/// The string set ends with two NULs (an empty set is just the two NULs).
fn find_double_nul(table: &[u8], from: usize) -> Option<usize> {
    (from..table.len().saturating_sub(1)).find(|&i| table[i] == 0 && table[i + 1] == 0)
}

fn string_at(formatted: &[u8], strings: &[&[u8]], offset: usize) -> Option<String> {
    let index = *formatted.get(offset)? as usize;
    let bytes = strings.get(index.checked_sub(1)?)?;
    let s = String::from_utf8_lossy(bytes).trim().to_string();
    (!s.is_empty()).then_some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structure(kind: u8, formatted: &[u8], strings: &[&str]) -> Vec<u8> {
        let mut v = vec![kind, (formatted.len() + 4) as u8, 0, 0];
        v.extend_from_slice(formatted);
        if strings.is_empty() {
            v.push(0);
        }
        for s in strings {
            v.extend_from_slice(s.as_bytes());
            v.push(0);
        }
        v.push(0);
        v
    }

    fn rsmb(structures: &[Vec<u8>]) -> Vec<u8> {
        let table: Vec<u8> = structures.concat();
        let mut v = vec![0, 3, 4, 0];
        v.extend_from_slice(&(table.len() as u32).to_le_bytes());
        v.extend(table);
        v
    }

    #[test]
    fn reads_model_service_tag_and_bios() {
        // Type 0: vendor=1, version=2, start segment (2 bytes), date=3.
        let bios = structure(0, &[1, 2, 0, 0xF0, 3], &["Dell Inc.", "1.31.0", "06/12/2025"]);
        // Type 1: manufacturer=1, product=2, version=0, serial=3.
        let system = structure(1, &[1, 2, 0, 3], &["Dell Inc.", "Precision 5560", "ABC1234"]);
        let end = structure(127, &[], &[]);
        let info = parse(&rsmb(&[bios, system, end]));
        assert_eq!(
            info,
            DeviceInfo {
                manufacturer: Some("Dell Inc.".into()),
                model: Some("Precision 5560".into()),
                service_tag: Some("ABC1234".into()),
                bios_version: Some("1.31.0".into()),
                bios_date: Some("06/12/2025".into()),
            }
        );
    }

    #[test]
    fn skips_other_structures_and_structures_without_strings() {
        let other = structure(4, &[0, 0, 0, 0], &[]);
        let system = structure(1, &[0, 1, 0, 0], &["Precision 5560"]);
        let info = parse(&rsmb(&[other, system]));
        assert_eq!(info.model.as_deref(), Some("Precision 5560"));
        assert_eq!(info.service_tag, None);
        assert_eq!(info.manufacturer, None);
    }

    #[test]
    fn blank_and_truncated_input_yield_nothing() {
        assert_eq!(parse(&[]), DeviceInfo::default());
        assert_eq!(parse(&[0, 3, 4, 0, 50, 0, 0, 0, 1, 99]), DeviceInfo::default());
        let system = structure(1, &[1, 2, 0, 3], &["  ", "Precision 5560", ""]);
        let info = parse(&rsmb(&[system]));
        assert_eq!(info.manufacturer, None);
        assert_eq!(info.model.as_deref(), Some("Precision 5560"));
    }
}
