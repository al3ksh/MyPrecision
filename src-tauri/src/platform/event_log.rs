//! Event log queries through wevtapi; events come back as rendered XML.

use windows::Win32::System::EventLog::{EVT_HANDLE, EvtClose, EvtNext, EvtQuery, EvtRender};
use windows::core::HSTRING;

const EVT_QUERY_CHANNEL_PATH: u32 = 0x1;
const EVT_QUERY_REVERSE_DIRECTION: u32 = 0x200;
const EVT_RENDER_EVENT_XML: u32 = 1;

/// Newest-first events of `channel` matching the XPath `query`, at most `max`.
pub fn query(channel: &str, xpath: &str, max: usize) -> windows::core::Result<Vec<String>> {
    // SAFETY: every handle EvtQuery/EvtNext returns is closed exactly once below.
    let results = unsafe {
        EvtQuery(
            None,
            &HSTRING::from(channel),
            &HSTRING::from(xpath),
            EVT_QUERY_CHANNEL_PATH | EVT_QUERY_REVERSE_DIRECTION,
        )
    }?;
    let mut out = Vec::new();
    let mut batch = [0isize; 32];
    while out.len() < max {
        let mut returned = 0u32;
        // SAFETY: `batch` holds as many handles as its length reports.
        if unsafe { EvtNext(results, &mut batch, 1000, 0, &mut returned) }.is_err() {
            break;
        }
        for &raw in &batch[..returned as usize] {
            let event = EVT_HANDLE(raw);
            if out.len() < max
                && let Some(xml) = render(event)
            {
                out.push(xml);
            }
            // SAFETY: each event handle is closed once.
            unsafe {
                let _ = EvtClose(event);
            }
        }
    }
    // SAFETY: closed once.
    unsafe {
        let _ = EvtClose(results);
    }
    Ok(out)
}

fn render(event: EVT_HANDLE) -> Option<String> {
    let (mut used, mut props) = (0u32, 0u32);
    // SAFETY: a null buffer only reports the required size (and fails with ERROR_INSUFFICIENT_BUFFER).
    let _ = unsafe { EvtRender(None, event, EVT_RENDER_EVENT_XML, 0, None, &mut used, &mut props) };
    if used == 0 {
        return None;
    }
    let mut buf = vec![0u16; (used as usize).div_ceil(2)];
    // SAFETY: `buf` is at least `used` bytes long.
    unsafe {
        EvtRender(
            None,
            event,
            EVT_RENDER_EVENT_XML,
            (buf.len() * 2) as u32,
            Some(buf.as_mut_ptr().cast()),
            &mut used,
            &mut props,
        )
    }
    .ok()?;
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}
