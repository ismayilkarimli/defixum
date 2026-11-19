//! What a country's forest holds: its trees, the landmarks and small things that stand in it,
//! water, flowers and the birds overhead. Each pause draws a few at random from the whole
//! country, so one screen can hold Tallinn's spires and a Saaremaa windmill together.
//! A country without its own pool keeps the plain spruce forest.

use forest::marks::Mark;
use forest::{Rng, Tree};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Water {
    /// open sea to the horizon, in place of the far ridge
    Sea,
    /// a lake at the foot of the far ridge
    Lake,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Bird {
    Gull,
    Swallow,
    Stork,
    Crane,
}

/// A landmark on one of the tree lines.
pub struct Placed {
    pub layer: usize,
    pub mark: Mark,
    /// where along the screen, 0 to 1
    pub at: f64,
    /// its size, as a share of the screen height
    pub size: f64,
}

pub struct Scene {
    /// the tree mix of each tree line, far to near
    pub kinds: [&'static [(Tree, f64)]; 5],
    pub water: Option<Water>,
    pub marks: Vec<Placed>,
    pub flowers: bool,
    pub birds: &'static [(Bird, f64)],
}

const EE_FAR: &[(Tree, f64)] = &[(Tree::Spruce, 0.3), (Tree::Pine, 0.35), (Tree::Birch, 0.2), (Tree::Oak, 0.15)];
const EE_NEAR: &[(Tree, f64)] =
    &[(Tree::Spruce, 0.3), (Tree::Pine, 0.3), (Tree::Birch, 0.22), (Tree::Oak, 0.12), (Tree::Juniper, 0.06)];
const EE_MEADOW: &[(Tree, f64)] =
    &[(Tree::Juniper, 0.4), (Tree::Pine, 0.2), (Tree::Birch, 0.2), (Tree::Oak, 0.2)];
// the giants in front stay spruce: a birch that size reads as a ragged pole
const EE_GIANTS: &[(Tree, f64)] = &[];
const EE_BIRDS: &[(Bird, f64)] = &[(Bird::Swallow, 0.4), (Bird::Stork, 0.3), (Bird::Crane, 0.3)];

fn one<T: Copy>(items: &[T], r: &mut Rng) -> T {
    items[((r.next() * items.len() as f64) as usize).min(items.len() - 1)]
}

/// a spot on the left or right of the clearing, `from`..`to` in from that edge
fn side(r: &mut Rng, left: bool, from: f64, to: f64) -> f64 {
    let v = from + r.next() * (to - from);
    if left {
        v
    } else {
        1.0 - v
    }
}

fn estonia(seed: u32) -> Scene {
    let mut r = Rng::new(seed ^ 0x45_45);
    let roll = r.next();
    let water = if roll < 0.3 {
        Some(Water::Sea)
    } else if roll < 0.62 {
        Some(Water::Lake)
    } else {
        None
    };
    let left = r.next() < 0.5;
    let mut marks = vec![];
    // on the far ridge or shore
    let far = match water {
        Some(Water::Sea) => one(&[Mark::Tallinn, Mark::Tallinn, Mark::Lighthouse], &mut r),
        Some(Water::Lake) => one(&[Mark::Munamagi, Mark::Church, Mark::TartuRuin], &mut r),
        None => one(&[Mark::Tallinn, Mark::Munamagi, Mark::TartuRuin, Mark::Church], &mut r),
    };
    // across open water it stands on the horizon; over land, on the nearer ridge, out of the haze
    let far_layer = if water == Some(Water::Sea) { 0 } else { 1 };
    let far_size = match far {
        Mark::Tallinn => 0.15,
        Mark::Lighthouse => 0.1,
        Mark::Munamagi => 0.13,
        Mark::TartuRuin => 0.085,
        _ => 0.12,
    };
    marks.push(Placed { layer: far_layer, mark: far, at: side(&mut r, left, 0.16, 0.3), size: far_size });
    // windmills or a church across the fields, on the other side
    if r.next() < 0.6 {
        let mid = if r.next() < 0.7 { Mark::Windmills } else { Mark::Church };
        let size = if mid == Mark::Windmills { 0.09 } else { 0.12 };
        marks.push(Placed { layer: 2, mark: mid, at: side(&mut r, !left, 0.12, 0.26), size });
    }
    // and something close by
    if r.next() < 0.75 {
        let near = one(&[Mark::Swing, Mark::StorkNest, Mark::Boulder], &mut r);
        let size = match near {
            Mark::Swing => 0.16,
            Mark::StorkNest => 0.2,
            _ => 0.05,
        };
        let near_left = r.next() < 0.5;
        marks.push(Placed { layer: 3, mark: near, at: side(&mut r, near_left, 0.1, 0.2), size });
    }
    let meadow = r.next() < 0.35;
    Scene {
        kinds: [EE_FAR, EE_FAR, EE_NEAR, if meadow { EE_MEADOW } else { EE_NEAR }, EE_GIANTS],
        water,
        marks,
        flowers: r.next() < 0.55,
        birds: EE_BIRDS,
    }
}

/// The scene for a country code ("EE"), or nothing for the plain forest.
pub fn plan(country: Option<&str>, seed: u32) -> Option<Scene> {
    match country? {
        "EE" => Some(estonia(seed)),
        _ => None,
    }
}
