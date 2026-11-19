//! Landmarks and small things that stand on a tree line, in its colour. Each takes the point on
//! the ground it stands on (`x`, `y`) and a size `s` (about its height, in px) and returns one
//! SVG path; `width` says how much of the line it takes, so it can be placed clear of the edges.

use crate::shape::{arch, bar, block, blob, gable, hole, mound, solid, taper};
use crate::Rng;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    /// Tallinn's Old Town: Toompea on its hill, the spires of St Olaf's, St Nicholas' and the
    /// Town Hall, wall towers and Fat Margaret by the harbour.
    Tallinn,
    /// A row of Saaremaa post mills, as at Angla.
    Windmills,
    /// A coastal lighthouse with its keeper's house.
    Lighthouse,
    /// The observation tower on Suur Munamägi, the highest hill in the Baltics.
    Munamagi,
    /// The ruin of Tartu's cathedral on Toome Hill: twin towers and a roofless Gothic nave.
    TartuRuin,
    /// A village church with a tall west spire.
    Church,
    /// The big wooden village swing (kiik).
    Swing,
    /// A white stork on its nest atop a pole.
    StorkNest,
    /// A glacial boulder (rändrahn) in a field.
    Boulder,
}

impl Mark {
    /// How wide it stands, in units of its size.
    pub fn width(self) -> f64 {
        match self {
            Mark::Tallinn => 3.1,
            Mark::Windmills => 2.6,
            Mark::Lighthouse => 0.8,
            Mark::Munamagi => 2.4,
            Mark::TartuRuin => 1.7,
            Mark::Church => 1.1,
            Mark::Swing => 1.1,
            Mark::StorkNest => 0.45,
            Mark::Boulder => 1.9,
        }
    }

    pub fn draw(self, x: f64, y: f64, s: f64, r: &mut Rng) -> String {
        match self {
            Mark::Tallinn => tallinn(x, y, s, r),
            Mark::Windmills => windmills(x, y, s, r),
            Mark::Lighthouse => lighthouse(x, y, s),
            Mark::Munamagi => munamagi(x, y, s, r),
            Mark::TartuRuin => tartu_ruin(x, y, s),
            Mark::Church => church(x, y, s, r),
            Mark::Swing => swing(x, y, s),
            Mark::StorkNest => stork_nest(x, y, s, r),
            Mark::Boulder => mound(x, y + s * 0.05, s * 0.9, s, 0.07, r),
        }
    }
}

/// A row of gabled merchant houses between `x0` and `x1`.
fn houses(x0: f64, x1: f64, y: f64, s: f64, r: &mut Rng) -> String {
    let mut d = String::new();
    let mut x = x0;
    while x < x1 {
        let w = s * (0.06 + r.next() * 0.04);
        let h = s * (0.12 + r.next() * 0.09);
        d.push_str(&block(x + w / 2.0, y, w, h));
        d.push_str(&gable(x + w / 2.0, y - h + 0.5, w * 1.04, w * (0.9 + r.next() * 0.5)));
        x += w * 0.98;
    }
    d
}

/// A tower: a square shaft, a short drum, then a needle spire.
fn spire(x: f64, y: f64, w: f64, shaft: f64, total: f64) -> String {
    let mut d = block(x, y, w, shaft);
    d.push_str(&taper(x, y - shaft + 0.5, w, w * 0.7, total * 0.05));
    let at = shaft + total * 0.05;
    d.push_str(&gable(x, y - at + 0.5, w * 0.72, total - at));
    d
}

fn tallinn(x: f64, y: f64, s: f64, r: &mut Rng) -> String {
    let l = x - s * 1.55;
    let u = |k: f64| l + s * k;
    // Toompea: the limestone hill with the castle and the Dome Church
    let hy = y - s * 0.15;
    let mut d = solid(&[(u(0.0), y + 1.0), (u(0.1), hy), (u(1.02), hy), (u(1.16), y + 1.0)]);
    d.push_str(&block(u(0.36), hy + 0.5, s * 0.46, s * 0.11));
    // Tall Hermann with its flag
    d.push_str(&block(u(0.18), hy + 0.5, s * 0.1, s * 0.38));
    d.push_str(&block(u(0.18), hy - s * 0.38 + 0.5, s * 0.115, s * 0.025));
    d.push_str(&bar((u(0.18), hy - s * 0.4), (u(0.18), hy - s * 0.52), s * 0.01));
    d.push_str(&solid(&[(u(0.18), hy - s * 0.52), (u(0.25), hy - s * 0.5), (u(0.18), hy - s * 0.475)]));
    // the Dome Church: a nave and a baroque tower with a bell cap and lantern
    d.push_str(&block(u(0.8), hy + 0.5, s * 0.26, s * 0.15));
    d.push_str(&gable(u(0.8), hy - s * 0.15 + 0.5, s * 0.27, s * 0.08));
    d.push_str(&block(u(0.64), hy + 0.5, s * 0.085, s * 0.31));
    let cap = hy - s * 0.31;
    d.push_str(&solid(&[
        (u(0.64) - s * 0.05, cap + 0.5),
        (u(0.64) - s * 0.045, cap - s * 0.03),
        (u(0.64) - s * 0.02, cap - s * 0.07),
        (u(0.64) + s * 0.02, cap - s * 0.07),
        (u(0.64) + s * 0.045, cap - s * 0.03),
        (u(0.64) + s * 0.05, cap + 0.5),
    ]));
    d.push_str(&block(u(0.64), cap - s * 0.07 + 0.5, s * 0.025, s * 0.05));
    d.push_str(&gable(u(0.64), cap - s * 0.12 + 0.5, s * 0.03, s * 0.07));
    d.push_str(&houses(u(0.88), u(1.0), hy + 0.5, s, r));
    // the lower town within its wall
    d.push_str(&block(u(2.0), y + 0.5, s * 1.9, s * 0.08));
    d.push_str(&houses(u(1.1), u(2.95), y + 0.5, s, r));
    // St Nicholas'
    d.push_str(&block(u(1.43), y + 0.5, s * 0.24, s * 0.18));
    d.push_str(&gable(u(1.43), y - s * 0.18 + 0.5, s * 0.25, s * 0.1));
    d.push_str(&spire(u(1.3), y + 0.5, s * 0.08, s * 0.32, s * 0.64));
    // the Town Hall, crenellated, with its slim tower
    d.push_str(&block(u(1.82), y + 0.5, s * 0.28, s * 0.19));
    for k in 0..6 {
        d.push_str(&block(u(1.7) + s * 0.05 * k as f64, y - s * 0.19 + 0.5, s * 0.025, s * 0.025));
    }
    d.push_str(&block(u(1.86), y - s * 0.18, s * 0.045, s * 0.14));
    d.push_str(&taper(u(1.86), y - s * 0.32 + 0.5, s * 0.055, s * 0.045, s * 0.06));
    d.push_str(&gable(u(1.86), y - s * 0.38 + 0.5, s * 0.035, s * 0.2));
    // St Olaf's, once the tallest building in the world
    d.push_str(&block(u(2.32), y + 0.5, s * 0.3, s * 0.2));
    d.push_str(&gable(u(2.32), y - s * 0.2 + 0.5, s * 0.31, s * 0.09));
    d.push_str(&spire(u(2.5), y + 0.5, s * 0.11, s * 0.42, s));
    // wall towers with their cone roofs, and Fat Margaret by the harbour
    for k in [1.16, 2.08, 2.72] {
        d.push_str(&block(u(k), y + 0.5, s * 0.06, s * 0.2));
        d.push_str(&gable(u(k), y - s * 0.2 + 0.5, s * 0.08, s * 0.12));
    }
    d.push_str(&block(u(3.0), y + 0.5, s * 0.2, s * 0.15));
    d.push_str(&gable(u(3.0), y - s * 0.15 + 0.5, s * 0.21, s * 0.04));
    d
}

/// One post mill: a body turned on a post over a trestle, a tail pole to the ground behind,
/// and four lattice sails.
fn post_mill(x: f64, y: f64, s: f64, turn: f64, face: f64) -> String {
    let mut d = bar((x - s * 0.13, y + 1.0), (x, y - s * 0.3), s * 0.03);
    d.push_str(&bar((x + s * 0.13, y + 1.0), (x, y - s * 0.3), s * 0.03));
    d.push_str(&block(x, y + 1.0, s * 0.04, s * 0.33));
    d.push_str(&block(x, y - s * 0.3, s * 0.25, s * 0.3));
    d.push_str(&gable(x, y - s * 0.6 + 0.5, s * 0.29, s * 0.12));
    d.push_str(&bar((x - face * s * 0.08, y - s * 0.33), (x - face * s * 0.46, y + 1.0), s * 0.025));
    let hub = (x + face * s * 0.02, y - s * 0.5);
    for k in 0..4 {
        let a = turn + std::f64::consts::FRAC_PI_2 * k as f64;
        let (c, sn) = (a.cos(), a.sin());
        let at = |along: f64, off: f64| (hub.0 + c * along - sn * off, hub.1 + sn * along + c * off);
        d.push_str(&bar(hub, at(s * 0.5, 0.0), s * 0.022));
        // the sail frame beside the stock, with its lattice showing as two openings
        let (i0, i1, wd) = (s * 0.1, s * 0.5, s * 0.085);
        d.push_str(&solid(&[at(i0, 0.0), at(i1, 0.0), at(i1, wd), at(i0, wd)]));
        let gap = s * 0.016;
        for (a0, a1) in [(i0 + gap, (i0 + i1) / 2.0 - gap / 2.0), ((i0 + i1) / 2.0 + gap / 2.0, i1 - gap)] {
            d.push_str(&hole(&[at(a0, gap), at(a1, gap), at(a1, wd - gap), at(a0, wd - gap)]));
        }
    }
    d
}

fn windmills(x: f64, y: f64, s: f64, r: &mut Rng) -> String {
    let n = 2 + (r.next() * 3.0).floor() as usize;
    let gap = s * 0.82;
    let first = x - gap * (n - 1) as f64 / 2.0;
    let face = if r.next() < 0.5 { 1.0 } else { -1.0 };
    (0..n)
        .map(|k| {
            let sz = s * (0.88 + r.next() * 0.16);
            post_mill(first + gap * k as f64, y, sz, r.next() * std::f64::consts::FRAC_PI_2, face)
        })
        .collect()
}

fn lighthouse(x: f64, y: f64, s: f64) -> String {
    let mut d = taper(x, y + 1.0, s * 0.18, s * 0.11, s * 0.72);
    d.push_str(&block(x, y - s * 0.72 + 0.5, s * 0.17, s * 0.025));
    d.push_str(&block(x, y - s * 0.745 + 0.5, s * 0.09, s * 0.1));
    d.push_str(&gable(x, y - s * 0.845 + 0.5, s * 0.12, s * 0.07));
    d.push_str(&bar((x, y - s * 0.9), (x, y - s * 0.97), s * 0.012));
    d.push_str(&block(x + s * 0.3, y + 1.0, s * 0.28, s * 0.12));
    d.push_str(&gable(x + s * 0.3, y - s * 0.12 + 0.5, s * 0.32, s * 0.1));
    d
}

fn munamagi(x: f64, y: f64, s: f64, r: &mut Rng) -> String {
    let mut d = mound(x, y + 1.0, s * 1.2, s * 0.2, 0.03, r);
    let hy = y - s * 0.19;
    d.push_str(&taper(x, hy + 1.0, s * 0.13, s * 0.09, s * 0.55));
    d.push_str(&taper(x, hy - s * 0.55 + 0.5, s * 0.14, s * 0.18, s * 0.04));
    d.push_str(&block(x, hy - s * 0.59 + 0.5, s * 0.18, s * 0.13));
    d.push_str(&block(x, hy - s * 0.72 + 0.5, s * 0.2, s * 0.02));
    d.push_str(&bar((x, hy - s * 0.74), (x, hy - s * 0.86), s * 0.012));
    d
}

fn tartu_ruin(x: f64, y: f64, s: f64) -> String {
    let mut d = String::new();
    // the west front: two towers, cut short, with the gable wall between them
    for tx in [x - s * 0.55, x - s * 0.2] {
        d.push_str(&block(tx, y + 1.0, s * 0.19, s));
        for (wy, wh) in [(0.52, 0.13), (0.76, 0.13)] {
            d.push_str(&hole(&arch(tx, y - s * wy, s * 0.045, s * wh)));
        }
    }
    d.push_str(&block(x - s * 0.375, y + 1.0, s * 0.18, s * 0.66));
    d.push_str(&hole(&arch(x - s * 0.375, y - s * 0.12, s * 0.09, s * 0.38)));
    // the roofless nave: a broken top edge and tall pointed windows
    let top = [0.5, 0.47, 0.5, 0.44, 0.38, 0.42, 0.35, 0.3];
    let mut wall = vec![(x - s * 0.12, y + 1.0)];
    for (k, t) in top.iter().enumerate() {
        let wx = x - s * 0.12 + s * 1.0 * k as f64 / (top.len() - 1) as f64;
        wall.push((wx, y - s * t));
    }
    wall.push((x + s * 0.88, y + 1.0));
    d.push_str(&solid(&wall));
    for k in 0..4 {
        d.push_str(&hole(&arch(x + s * (0.06 + 0.2 * k as f64), y - s * 0.07, s * 0.085, s * 0.24)));
    }
    d
}

fn church(x: f64, y: f64, s: f64, r: &mut Rng) -> String {
    let side = if r.next() < 0.5 { 1.0 } else { -1.0 };
    let nave = x + side * s * 0.36;
    let mut d = block(nave, y + 1.0, s * 0.56, s * 0.24);
    d.push_str(&gable(nave, y - s * 0.24 + 0.5, s * 0.62, s * 0.18));
    d.push_str(&block(x + side * s * 0.72, y + 1.0, s * 0.2, s * 0.19));
    d.push_str(&gable(x + side * s * 0.72, y - s * 0.19 + 0.5, s * 0.24, s * 0.12));
    d.push_str(&block(x, y + 1.0, s * 0.17, s * 0.46));
    d.push_str(&block(x, y - s * 0.46 + 0.5, s * 0.19, s * 0.02));
    d.push_str(&gable(x, y - s * 0.48 + 0.5, s * 0.15, s * 0.5));
    d
}

fn swing(x: f64, y: f64, s: f64) -> String {
    let mut d = String::new();
    for px in [x - s * 0.45, x + s * 0.45] {
        d.push_str(&bar((px - s * 0.14, y + 1.0), (px, y - s), s * 0.035));
        d.push_str(&bar((px + s * 0.14, y + 1.0), (px, y - s), s * 0.035));
    }
    d.push_str(&bar((x - s * 0.5, y - s), (x + s * 0.5, y - s), s * 0.045));
    for rx in [x - s * 0.27, x + s * 0.27] {
        d.push_str(&bar((rx, y - s), (rx, y - s * 0.17), s * 0.018));
    }
    d.push_str(&bar((x - s * 0.34, y - s * 0.17), (x + s * 0.34, y - s * 0.17), s * 0.04));
    d
}

fn stork_nest(x: f64, y: f64, s: f64, r: &mut Rng) -> String {
    let mut d = bar((x, y + 1.0), (x, y - s * 0.78), s * 0.035);
    d.push_str(&solid(&blob(x, y - s * 0.8, s * 0.2, s * 0.055, 0.15, 14, r)));
    let face = if r.next() < 0.5 { 1.0 } else { -1.0 };
    let f = |dx: f64| x + face * dx;
    d.push_str(&solid(&blob(f(0.02 * s), y - s * 0.89, s * 0.075, s * 0.045, 0.05, 12, r)));
    d.push_str(&bar((f(-0.03 * s), y - s * 0.9), (f(-0.06 * s), y - s * 1.0), s * 0.022));
    d.push_str(&solid(&blob(f(-0.065 * s), y - s * 1.01, s * 0.022, s * 0.018, 0.0, 8, r)));
    d.push_str(&bar((f(-0.08 * s), y - s * 1.01), (f(-0.15 * s), y - s * 0.985), s * 0.012));
    d
}
