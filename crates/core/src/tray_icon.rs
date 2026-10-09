//! Tray icon pixels and flyout toggle debounce.

use crate::dell::ThermalMode;

const WHITE: [u8; 3] = [0xff, 0xff, 0xff];
const BLACK: [u8; 3] = [0x1c, 0x1c, 0x1c];

/// Gauge geometry on a unit square (y down): a 270° arc open at the bottom, a needle, a hub.
const CENTER: (f32, f32) = (0.5, 0.56);
const ARC_R: f32 = 0.40;
const NEEDLE_R: f32 = 0.25;
const STROKE: f32 = 0.10;
const HUB_R: f32 = 0.10;
const START_DEG: f32 = 225.0;
const SWEEP_DEG: f32 = 270.0;
/// Samples per pixel side for anti-aliasing.
const SS: usize = 4;

/// Where the needle sits for each thermal mode, from Quiet (left) to Ultra Performance (right).
pub fn needle_fraction(mode: Option<ThermalMode>) -> f32 {
    match mode {
        Some(ThermalMode::Quiet) => 0.12,
        Some(ThermalMode::Cool) => 0.36,
        Some(ThermalMode::Optimized) => 0.62,
        Some(ThermalMode::UltraPerformance) => 0.9,
        None => 0.5,
    }
}

/// Monochrome glyph colour that reads on the taskbar, like the system's own tray icons.
pub fn icon_color(light_taskbar: bool) -> [u8; 3] {
    if light_taskbar { BLACK } else { WHITE }
}

/// Unit vector at `deg` (counter-clockwise from +x) in y-down coordinates.
fn dir(deg: f32) -> (f32, f32) {
    let r = deg.to_radians();
    (r.cos(), -r.sin())
}

fn dist_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / (abx * abx + aby * aby)).clamp(0.0, 1.0);
    (p.0 - a.0 - t * abx).hypot(p.1 - a.1 - t * aby)
}

/// Signed distance to the glyph: negative inside.
fn gauge_distance(p: (f32, f32), fraction: f32) -> f32 {
    let (dx, dy) = (p.0 - CENTER.0, p.1 - CENTER.1);
    let at = |deg: f32, r: f32| (CENTER.0 + dir(deg).0 * r, CENTER.1 + dir(deg).1 * r);
    let end_deg = START_DEG - SWEEP_DEG;
    // The gap is the bottom wedge between the two ends; everything else is on the arc.
    let deg = (-dy).atan2(dx).to_degrees();
    let in_gap = deg < end_deg && deg > START_DEG - 360.0;
    let arc = if in_gap {
        let (a, b) = (at(START_DEG, ARC_R), at(end_deg, ARC_R));
        (p.0 - a.0).hypot(p.1 - a.1).min((p.0 - b.0).hypot(p.1 - b.1)) - STROKE / 2.0
    } else {
        (dx.hypot(dy) - ARC_R).abs() - STROKE / 2.0
    };
    let tip = at(START_DEG - SWEEP_DEG * fraction.clamp(0.0, 1.0), NEEDLE_R);
    let needle = dist_to_segment(p, CENTER, tip) - STROKE / 2.0;
    let hub = dx.hypot(dy) - HUB_R;
    arc.min(needle).min(hub)
}

/// RGBA `size`×`size`: a gauge whose needle sits at `fraction` (0 = left, 1 = right), supersampled.
pub fn render_gauge_icon(color: [u8; 3], fraction: f32, size: u32) -> Vec<u8> {
    let n = size as usize;
    let step = 1.0 / (n * SS) as f32;
    let mut out = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let mut hits = 0;
            for sy in 0..SS {
                for sx in 0..SS {
                    let p = (((x * SS + sx) as f32 + 0.5) * step, ((y * SS + sy) as f32 + 0.5) * step);
                    if gauge_distance(p, fraction) <= 0.0 {
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                let i = (y * n + x) * 4;
                out[i..i + 3].copy_from_slice(&color);
                out[i + 3] = (hits * 255 / (SS * SS)) as u8;
            }
        }
    }
    out
}

/// Clicking the tray icon while the flyout is open first blurs the flyout (which hides it),
/// then delivers the click — which must not reopen it. The blur lands near the button press,
/// however long the button is then held, so the window is measured from the press.
#[derive(Debug, Default)]
pub struct ToggleGuard {
    last_press_ms: Option<i64>,
    last_hide_ms: Option<i64>,
}

impl ToggleGuard {
    pub const WINDOW_MS: i64 = 300;

    pub fn on_pressed(&mut self, now_ms: i64) {
        self.last_press_ms = Some(now_ms);
    }

    pub fn on_hidden(&mut self, now_ms: i64) {
        self.last_hide_ms = Some(now_ms);
    }

    /// Called on button release.
    pub fn should_open(&mut self, now_ms: i64) -> bool {
        let pressed = self.last_press_ms.take().unwrap_or(now_ms);
        !matches!(self.last_hide_ms.take(), Some(t) if t > pressed.min(now_ms) - Self::WINDOW_MS)
    }
}

/// On show, focus bounces between the flyout and its WebView2 child: `Focused(false)` is
/// followed by `Focused(true)` within a millisecond. Only a blur that lasts means the user
/// clicked elsewhere.
#[derive(Debug, Default)]
pub struct BlurDebounce {
    blurred_since_ms: Option<i64>,
}

impl BlurDebounce {
    pub const SETTLE_MS: i64 = 150;

    pub fn on_focus(&mut self, focused: bool, now_ms: i64) {
        self.blurred_since_ms = if focused { None } else { self.blurred_since_ms.or(Some(now_ms)) };
    }

    /// Ask again `SETTLE_MS` after a blur: true if focus never came back.
    pub fn should_hide(&self, now_ms: i64) -> bool {
        self.blurred_since_ms.is_some_and(|t| now_ms - t >= Self::SETTLE_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(px: &[u8]) -> impl Iterator<Item = u8> + '_ {
        px.as_chunks::<4>().0.iter().map(|p| p[3])
    }

    /// Opaque weight in the left and right halves.
    fn halves(px: &[u8], size: usize) -> (u32, u32) {
        let (mut l, mut r) = (0, 0);
        for (i, a) in alpha(px).enumerate() {
            if i % size < size / 2 { l += u32::from(a) } else { r += u32::from(a) }
        }
        (l, r)
    }

    #[test]
    fn gauge_has_expected_size() {
        for size in [16, 20, 24, 32] {
            assert_eq!(render_gauge_icon(WHITE, 0.5, size).len(), (size * size * 4) as usize);
        }
    }

    #[test]
    fn gauge_edges_are_antialiased() {
        let px = render_gauge_icon(WHITE, 0.5, 16);
        assert!(alpha(&px).any(|a| a > 0 && a < 255), "expected partially covered edge pixels");
        assert!(alpha(&px).any(|a| a == 255), "expected fully covered stroke pixels");
    }

    #[test]
    fn gauge_is_one_colour_on_transparent() {
        let px = render_gauge_icon([10, 20, 30], 0.3, 24);
        assert_eq!(&px[..4], &[0, 0, 0, 0], "corner stays transparent");
        assert!(px.as_chunks::<4>().0.iter().filter(|p| p[3] > 0).all(|p| p[..3] == [10, 20, 30]));
    }

    #[test]
    fn needle_points_left_at_zero_and_right_at_one() {
        let (l, r) = halves(&render_gauge_icon(WHITE, 0.0, 32), 32);
        assert!(l > r, "fraction 0 leans left ({l} vs {r})");
        let (l, r) = halves(&render_gauge_icon(WHITE, 1.0, 32), 32);
        assert!(r > l, "fraction 1 leans right ({l} vs {r})");
    }

    #[test]
    fn needle_orders_modes_quiet_to_ultra() {
        let f = |m| needle_fraction(Some(m));
        assert!(f(ThermalMode::Quiet) < f(ThermalMode::Cool));
        assert!(f(ThermalMode::Cool) < f(ThermalMode::Optimized));
        assert!(f(ThermalMode::Optimized) < f(ThermalMode::UltraPerformance));
        assert_eq!(needle_fraction(None), 0.5);
        assert_ne!(
            render_gauge_icon(WHITE, f(ThermalMode::Quiet), 16),
            render_gauge_icon(WHITE, f(ThermalMode::Cool), 16)
        );
    }

    #[test]
    fn colour_follows_taskbar_theme() {
        assert_eq!(icon_color(false), WHITE);
        assert_eq!(icon_color(true), BLACK);
    }

    #[test]
    fn click_right_after_blur_hide_does_not_reopen() {
        let mut g = ToggleGuard::default();
        g.on_hidden(1000);
        assert!(!g.should_open(1150));
        assert!(g.should_open(1400));
    }

    #[test]
    fn long_press_that_blurred_the_flyout_does_not_reopen_it() {
        let mut g = ToggleGuard::default();
        g.on_pressed(1000);
        g.on_hidden(1010); // blur on mouse down
        assert!(!g.should_open(1800)); // released much later
    }

    #[test]
    fn click_long_after_an_earlier_hide_opens() {
        let mut g = ToggleGuard::default();
        g.on_hidden(1000);
        g.on_pressed(5000);
        assert!(g.should_open(5900));
    }

    #[test]
    fn focus_bounce_right_after_show_does_not_hide() {
        // Observed on show: Focused(true), Focused(false), Focused(true) within ~1 ms.
        let mut b = BlurDebounce::default();
        b.on_focus(true, 1000);
        b.on_focus(false, 1001);
        b.on_focus(true, 1001);
        assert!(!b.should_hide(1001 + BlurDebounce::SETTLE_MS));
    }

    #[test]
    fn blur_that_stays_hides_after_settle() {
        let mut b = BlurDebounce::default();
        b.on_focus(true, 1000);
        b.on_focus(false, 2000);
        assert!(!b.should_hide(2000 + BlurDebounce::SETTLE_MS - 1));
        assert!(b.should_hide(2000 + BlurDebounce::SETTLE_MS));
    }
}
