//! Small polygon helpers for silhouettes. Every outline is wound clockwise on screen (as the
//! tree lines are), so overlapping parts of one path fill as one; holes are wound the other way.

use crate::Rng;

pub type Pt = (f64, f64);

/// Twice the signed area; positive is clockwise on screen (y grows downward).
fn area2(pts: &[Pt]) -> f64 {
    let n = pts.len();
    (0..n).map(|i| {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        a.0 * b.1 - b.0 * a.1
    })
    .sum()
}

fn emit(pts: &[Pt], clockwise: bool) -> String {
    if pts.len() < 3 {
        return String::new();
    }
    let flip = (area2(pts) > 0.0) != clockwise;
    let mut d = String::with_capacity(pts.len() * 14);
    let mut first = true;
    let mut push = |p: &Pt| {
        d.push(if first { 'M' } else { 'L' });
        first = false;
        d.push_str(&format!("{:.1} {:.1}", p.0, p.1));
    };
    if flip {
        pts.iter().rev().for_each(&mut push);
    } else {
        pts.iter().for_each(&mut push);
    }
    d.push('Z');
    d
}

/// A filled outline.
pub fn solid(pts: &[Pt]) -> String {
    emit(pts, true)
}

/// A hole cut out of a filled outline in the same path.
pub fn hole(pts: &[Pt]) -> String {
    emit(pts, false)
}

/// Axis-aligned box from its bottom centre.
pub fn block(x: f64, y: f64, w: f64, h: f64) -> String {
    solid(&[(x - w / 2.0, y), (x - w / 2.0, y - h), (x + w / 2.0, y - h), (x + w / 2.0, y)])
}

/// A box narrowing toward the top, from its bottom centre.
pub fn taper(x: f64, y: f64, w0: f64, w1: f64, h: f64) -> String {
    solid(&[(x - w0 / 2.0, y), (x - w1 / 2.0, y - h), (x + w1 / 2.0, y - h), (x + w0 / 2.0, y)])
}

/// A straight stroke of some thickness, as a filled quad.
pub fn bar(a: Pt, b: Pt, t: f64) -> String {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let (nx, ny) = (-dy / len * t / 2.0, dx / len * t / 2.0);
    solid(&[(a.0 + nx, a.1 + ny), (b.0 + nx, b.1 + ny), (b.0 - nx, b.1 - ny), (a.0 - nx, a.1 - ny)])
}

/// A rough ellipse: the outline of a clump of leaves, a rock, a bush.
pub fn blob(cx: f64, cy: f64, rx: f64, ry: f64, rough: f64, n: usize, r: &mut Rng) -> Vec<Pt> {
    let tau = std::f64::consts::TAU;
    let turn = r.next() * tau;
    (0..n)
        .map(|i| {
            let a = turn + tau * i as f64 / n as f64;
            let k = 1.0 + (r.next() - 0.5) * 2.0 * rough;
            (cx + a.cos() * rx * k, cy + a.sin() * ry * k)
        })
        .collect()
}

/// The top half of a rough ellipse standing on `y`: a mound, a boulder, a bush.
pub fn mound(cx: f64, y: f64, rx: f64, ry: f64, rough: f64, r: &mut Rng) -> String {
    let n = 14;
    let mut pts = vec![(cx - rx, y + 1.0)];
    for i in 0..=n {
        let a = std::f64::consts::PI * (1.0 + i as f64 / n as f64);
        let k = if i == 0 || i == n { 1.0 } else { 1.0 + (r.next() - 0.5) * 2.0 * rough };
        pts.push((cx + a.cos() * rx * k, y + a.sin() * ry * k));
    }
    pts.push((cx + rx, y + 1.0));
    solid(&pts)
}

/// A pointed (Gothic) arch from its bottom centre, as an outline: two arcs of radius `w`, each
/// centred on the opposite springing point.
pub fn arch(x: f64, y: f64, w: f64, h: f64) -> Vec<Pt> {
    let rise = w * 0.866;
    let spring = y - (h - rise).max(0.0);
    let pi = std::f64::consts::PI;
    let mut pts = vec![(x - w / 2.0, y)];
    for i in 0..=6 {
        let a = pi - (pi / 3.0) * i as f64 / 6.0;
        pts.push((x + w / 2.0 + w * a.cos(), spring - w * a.sin()));
    }
    for i in (0..6).rev() {
        let a = pi - (pi / 3.0) * i as f64 / 6.0;
        pts.push((x - w / 2.0 - w * a.cos(), spring - w * a.sin()));
    }
    pts.push((x + w / 2.0, y));
    pts
}

/// A pitched roof (or any triangle) over a span, its ridge `h` above `y`.
pub fn gable(x: f64, y: f64, w: f64, h: f64) -> String {
    solid(&[(x - w / 2.0, y), (x, y - h), (x + w / 2.0, y)])
}
