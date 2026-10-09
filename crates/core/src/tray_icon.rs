//! Tray icon pixels and flyout toggle debounce.

use crate::profile::{ActiveProfile, BatteryProfile};

const ACCENT: [u8; 3] = [0x4c, 0xc2, 0xff];
const WHITE: [u8; 3] = [0xf3, 0xf3, 0xf3];
const GREY: [u8; 3] = [0x9d, 0x9d, 0x9d];

/// RGBA `size`×`size`: a horizontal battery outline with a fill proportional to `fill_pct`.
pub fn render_battery_icon(color: [u8; 3], fill_pct: Option<u8>, size: u32) -> Vec<u8> {
    let n = size as usize;
    let unit = size as f32 / 32.0;
    let px = |v: f32| (v * unit).round() as usize;
    let stroke = px(2.0).max(1);
    // Layout on a 32-unit grid: body 1..27 × 7..25, terminal nub 27..30 × 12..20.
    let (bx0, bx1, by0, by1) = (px(1.0), px(27.0), px(7.0), px(25.0));
    let (nx1, ny0, ny1) = (px(30.0), px(12.0), px(20.0));
    let gap = stroke * 2;
    let inner_w = bx1.saturating_sub(bx0 + 2 * gap);
    let fill_w = fill_pct.map_or(0, |p| (inner_w as f32 * f32::from(p.min(100)) / 100.0).round() as usize);
    let fill_x1 = bx0 + gap + fill_w;

    let mut out = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let in_body = (bx0..bx1).contains(&x) && (by0..by1).contains(&y);
            let outline = in_body && (x < bx0 + stroke || x >= bx1 - stroke || y < by0 + stroke || y >= by1 - stroke);
            let nub = (bx1..nx1).contains(&x) && (ny0..ny1).contains(&y);
            let fill = (bx0 + gap..fill_x1).contains(&x) && (by0 + gap..by1 - gap).contains(&y);
            if outline || nub || fill {
                let i = (y * n + x) * 4;
                out[i..i + 3].copy_from_slice(&color);
                out[i + 3] = 255;
            }
        }
    }
    out
}

pub fn icon_color(active: Option<&ActiveProfile>) -> [u8; 3] {
    match active {
        Some(ActiveProfile::Known { profile: BatteryProfile::Home }) => ACCENT,
        Some(ActiveProfile::Known { profile: BatteryProfile::Storage }) => GREY,
        _ => WHITE,
    }
}

/// Clicking the tray icon while the flyout is open first blurs the flyout (which hides it),
/// then delivers the click — which must not reopen it.
#[derive(Debug, Default)]
pub struct ToggleGuard {
    last_hide_ms: Option<i64>,
}

impl ToggleGuard {
    pub const WINDOW_MS: i64 = 300;

    pub fn on_hidden(&mut self, now_ms: i64) {
        self.last_hide_ms = Some(now_ms);
    }

    pub fn should_open(&mut self, now_ms: i64) -> bool {
        !matches!(self.last_hide_ms.take(), Some(t) if now_ms - t < Self::WINDOW_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dell::ChargeCfg;

    fn opaque(px: &[u8]) -> usize {
        px.as_chunks::<4>().0.iter().filter(|p| p[3] > 0).count()
    }

    #[test]
    fn icon_has_expected_size() {
        assert_eq!(render_battery_icon([76, 194, 255], Some(50), 32).len(), 32 * 32 * 4);
    }

    #[test]
    fn icon_fill_grows_with_percent() {
        let low = opaque(&render_battery_icon([255, 255, 255], Some(20), 32));
        let high = opaque(&render_battery_icon([255, 255, 255], Some(80), 32));
        assert!(high > low, "80% ({high}) should cover more pixels than 20% ({low})");
    }

    #[test]
    fn colors_per_profile() {
        let known = |profile| icon_color(Some(&ActiveProfile::Known { profile }));
        assert_eq!(known(BatteryProfile::Home), [0x4c, 0xc2, 0xff]);
        assert_eq!(known(BatteryProfile::Campus), [0xf3, 0xf3, 0xf3]);
        assert_eq!(known(BatteryProfile::Storage), [0x9d, 0x9d, 0x9d]);
        let other = ActiveProfile::Other { cfg: ChargeCfg::Custom { start: 60, stop: 90 } };
        assert_eq!(icon_color(Some(&other)), [0xf3, 0xf3, 0xf3]);
    }

    #[test]
    fn click_right_after_blur_hide_does_not_reopen() {
        let mut g = ToggleGuard::default();
        g.on_hidden(1000);
        assert!(!g.should_open(1150));
        assert!(g.should_open(1400));
    }
}
