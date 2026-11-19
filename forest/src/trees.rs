//! Broadleaf and pine silhouettes beside the spruce. Each takes the spot a spruce would (`x`
//! its centre, `base` the ground, `w` and `h` its room) and returns one SVG path.

use crate::shape::{bar, blob, solid, taper};
use crate::Rng;

/// A trunk that leans a little and narrows upward, as a quad strip.
fn trunk(x: f64, base: f64, w0: f64, w1: f64, h: f64, lean: f64) -> String {
    let steps = 4;
    let mut left = vec![];
    let mut right = vec![];
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let cx = x + lean * t * t;
        let hw = (w0 + (w1 - w0) * t) / 2.0;
        left.push((cx - hw, base + 1.0 - h * t - if i == 0 { 0.0 } else { 0.0 }));
        right.push((cx + hw, base + 1.0 - h * t));
    }
    right.reverse();
    left.extend(right);
    solid(&left)
}

/// Scots pine: a long bare trunk, often a little crooked, carrying a high, irregular crown:
/// a loose cloud of needle masses over the top third, with a clump or two lower down on short
/// branches. Old pines over the bogs and dunes read by that high, broken crown.
pub fn pine(x: f64, base: f64, w: f64, h: f64, r: &mut Rng) -> String {
    let lean = (r.next() - 0.5) * w * 0.3;
    let at = |t: f64| x + lean * t * t;
    let mut d = trunk(x, base, w * 0.1, w * 0.045, h * 0.92, lean);
    let mid = 0.8 + r.next() * 0.04;
    let (half_w, half_h) = (w * (0.3 + r.next() * 0.1), h * (0.13 + r.next() * 0.04));
    let masses = 6 + (r.next() * 4.0).floor() as usize;
    for k in 0..masses {
        // spread through an oval, the first few filling its middle so the crown holds together
        let a = r.next() * std::f64::consts::TAU;
        let reach = if k < 2 { 0.15 } else { 0.45 + r.next() * 0.45 };
        let cx = at(mid) + a.cos() * half_w * reach;
        let cy = base - h * mid + a.sin() * half_h * reach;
        let rx = w * (0.13 + r.next() * 0.09);
        let ry = h * (0.05 + r.next() * 0.035);
        d.push_str(&solid(&blob(cx, cy, rx, ry, 0.18, 14, r)));
    }
    // a clump or two lower down on short branches
    let lower = (r.next() * 2.6).floor() as usize;
    let mut side = if r.next() < 0.5 { -1.0 } else { 1.0 };
    for k in 0..lower {
        let t = mid - 0.2 - k as f64 * 0.07 - r.next() * 0.04;
        let y = base - h * t;
        let cx = at(t) + side * w * (0.2 + r.next() * 0.1);
        let rx = w * (0.11 + r.next() * 0.06);
        d.push_str(&bar((at(t), y + h * 0.025), (cx, y), w * 0.03));
        d.push_str(&solid(&blob(cx, y, rx, h * 0.035, 0.18, 12, r)));
        side = -side;
    }
    d
}

/// Silver birch: a slim trunk bare for its lower third, then a loose, feathery oval crown,
/// narrow at the top, its lower edge hanging in drooping sprays.
pub fn birch(x: f64, base: f64, w: f64, h: f64, r: &mut Rng) -> String {
    let lean = (r.next() - 0.5) * w * 0.25;
    let mut d = trunk(x, base, w * 0.06, w * 0.028, h * 0.85, lean);
    let low = 0.3 + r.next() * 0.1;
    let cy = base - h * (low + 1.0) / 2.0;
    let ry = h * (1.0 - low) / 2.0;
    let rx = w * (0.28 + r.next() * 0.08);
    let cx = x + lean * 0.5;
    let n = 40;
    let tau = std::f64::consts::TAU;
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let a = tau * i as f64 / n as f64;
            // a soft egg: fuller below the middle, narrowing to the top
            let below = a.sin().max(0.0);
            let fringe = if i % 2 == 0 { 1.0 } else { 0.86 } * (0.93 + r.next() * 0.14);
            let px = cx + a.cos() * rx * (0.85 + 0.25 * below) * fringe;
            let droop = below * below * ry * 0.18 * (a.cos().abs() + 0.3);
            (px, cy + a.sin() * ry * fringe + droop)
        })
        .collect();
    d.push_str(&solid(&pts));
    d
}

/// Oak: a short, thick trunk and a broad crown of heavy rounded masses, wider than it is tall.
pub fn oak(x: f64, base: f64, w: f64, h: f64, r: &mut Rng) -> String {
    let lean = (r.next() - 0.5) * w * 0.08;
    let mut d = trunk(x, base, w * 0.13, w * 0.08, h * 0.5, lean);
    // two main limbs fork into the crown
    for side in [-1.0, 1.0] {
        d.push_str(&bar((x + lean * 0.25, base - h * 0.42), (x + side * w * 0.22, base - h * 0.62), w * 0.05));
    }
    let n = 6 + (r.next() * 3.0).floor() as usize;
    for k in 0..n {
        let a = std::f64::consts::PI * (0.08 + 0.84 * k as f64 / (n - 1) as f64);
        let cx = x - a.cos() * w * 0.32 + (r.next() - 0.5) * w * 0.06;
        let cy = base - h * 0.6 - a.sin() * h * 0.2;
        let rx = w * (0.17 + r.next() * 0.08);
        let ry = h * (0.15 + r.next() * 0.06);
        d.push_str(&solid(&blob(cx, cy, rx, ry, 0.14, 14, r)));
    }
    d.push_str(&solid(&blob(x, base - h * 0.68, w * 0.3, h * 0.22, 0.1, 16, r)));
    d
}

/// Juniper: a ragged dark column, sometimes leaning, as on the wooded meadows of Saaremaa.
pub fn juniper(x: f64, base: f64, w: f64, h: f64, r: &mut Rng) -> String {
    let lean = (r.next() - 0.5) * w * 0.5;
    let n = 9;
    let mut left = vec![(x - w * 0.2, base + 1.0)];
    let mut right = vec![(x + w * 0.2, base + 1.0)];
    for i in 1..n {
        let t = i as f64 / n as f64;
        // fullest a third of the way up, tapering to a blunt tip
        let half = w * 0.5 * (1.0 - ((t - 0.3) / 0.75).powi(2)).max(0.12) * (0.75 + r.next() * 0.45);
        let cx = x + lean * t;
        left.push((cx - half, base - h * t));
        right.push((cx + half * (0.75 + r.next() * 0.45), base - h * t));
    }
    left.push((x + lean, base - h));
    right.reverse();
    left.extend(right);
    let mut d = solid(&left);
    d.push_str(&taper(x, base + 1.0, w * 0.16, w * 0.12, h * 0.1));
    d
}
