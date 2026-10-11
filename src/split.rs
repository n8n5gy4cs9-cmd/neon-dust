//! Splitscreen layout - pure functions (no Bevy types) so the rules are unit-tested.
//! Ported from nordrep `core/split.rs` (itself from the NordCraft reference). Neon Dust uses 1-2 players;
//! the 3-4 player arrangements are kept for future gamepads. Rules: 1 player = fullscreen, 2 = top/bottom (or
//! left/right), 3–4 = quadrants (3 players leave the 4th quadrant empty). With 3–4 players a
//! slot keeps its quadrant when others join/leave so views never jump.


pub const MAX_PLAYERS: usize = 4;

/// Fractional rectangle of the window: all values in 0..=1, origin top-left.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl FRect {
    pub const FULL: FRect = FRect {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    };
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    /// Pixel rectangle `(x, y, w, h)` for a window of `ww` x `wh` pixels.
    pub fn to_px(self, ww: u32, wh: u32) -> (u32, u32, u32, u32) {
        let x0 = (self.x * ww as f32).round() as u32;
        let y0 = (self.y * wh as f32).round() as u32;
        let x1 = ((self.x + self.w) * ww as f32).round() as u32;
        let y1 = ((self.y + self.h) * wh as f32).round() as u32;
        (x0, y0, (x1 - x0).max(1), (y1 - y0).max(1))
    }
    pub fn aspect(self, ww: u32, wh: u32) -> f32 {
        (self.w * ww as f32) / (self.h * wh as f32).max(1.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TwoSplit {
    #[default]
    TopBottom,
    LeftRight,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Arrange {
    /// 2x2 quadrants.
    #[default]
    Grid,
    /// Wide big screen on top, rest side by side below.
    BigTop,
    /// Tall big screen on the left, rest stacked on the right.
    BigLeft,
    /// Vertical strips.
    Columns,
    /// Horizontal strips.
    Rows,
}

impl Arrange {
    pub const ALL: [Arrange; 5] = [
        Arrange::Grid,
        Arrange::BigTop,
        Arrange::BigLeft,
        Arrange::Columns,
        Arrange::Rows,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Arrange::Grid => "Quadrants",
            Arrange::BigTop => "Big Top",
            Arrange::BigLeft => "Big Left",
            Arrange::Columns => "Columns",
            Arrange::Rows => "Rows",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SplitOpts {
    pub two: TwoSplit,
    pub multi: Arrange,
    /// Slot that gets the big screen for Big* arrangements.
    pub main: usize,
}

impl Default for SplitOpts {
    fn default() -> Self {
        Self {
            two: TwoSplit::TopBottom,
            multi: Arrange::Grid,
            main: 0,
        }
    }
}

fn quadrant(i: usize) -> FRect {
    FRect::new((i % 2) as f32 * 0.5, (i / 2) as f32 * 0.5, 0.5, 0.5)
}

/// Rectangle per slot for the given active mask. `None` for inactive slots.
pub fn layout(active: [bool; MAX_PLAYERS], o: SplitOpts) -> [Option<FRect>; MAX_PLAYERS] {
    let slots: Vec<usize> = (0..MAX_PLAYERS).filter(|&i| active[i]).collect();
    let n = slots.len();
    let mut out = [None; MAX_PLAYERS];
    match n {
        0 => {}
        1 => out[slots[0]] = Some(FRect::FULL),
        2 => {
            let (a, b) = match o.two {
                TwoSplit::TopBottom => (FRect::new(0.0, 0.0, 1.0, 0.5), FRect::new(0.0, 0.5, 1.0, 0.5)),
                TwoSplit::LeftRight => (FRect::new(0.0, 0.0, 0.5, 1.0), FRect::new(0.5, 0.0, 0.5, 1.0)),
            };
            out[slots[0]] = Some(a);
            out[slots[1]] = Some(b);
        }
        _ => match o.multi {
            Arrange::Grid => {
                for &s in &slots {
                    out[s] = Some(quadrant(s));
                }
            }
            Arrange::Columns => {
                for (k, &s) in slots.iter().enumerate() {
                    out[s] = Some(FRect::new(k as f32 / n as f32, 0.0, 1.0 / n as f32, 1.0));
                }
            }
            Arrange::Rows => {
                for (k, &s) in slots.iter().enumerate() {
                    out[s] = Some(FRect::new(0.0, k as f32 / n as f32, 1.0, 1.0 / n as f32));
                }
            }
            Arrange::BigTop | Arrange::BigLeft => {
                let main = if active[o.main.min(3)] {
                    o.main.min(3)
                } else {
                    slots[0]
                };
                let rest: Vec<usize> = slots.iter().copied().filter(|&s| s != main).collect();
                let m = rest.len() as f32;
                if o.multi == Arrange::BigTop {
                    out[main] = Some(FRect::new(0.0, 0.0, 1.0, 0.6));
                    for (k, &s) in rest.iter().enumerate() {
                        out[s] = Some(FRect::new(k as f32 / m, 0.6, 1.0 / m, 0.4));
                    }
                } else {
                    out[main] = Some(FRect::new(0.0, 0.0, 0.66, 1.0));
                    for (k, &s) in rest.iter().enumerate() {
                        out[s] = Some(FRect::new(0.66, k as f32 / m, 0.34, 1.0 / m));
                    }
                }
            }
        },
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mask(n: usize) -> [bool; 4] {
        [n > 0, n > 1, n > 2, n > 3]
    }
    #[test]
    fn one_is_full() {
        assert_eq!(layout(mask(1), SplitOpts::default())[0], Some(FRect::FULL));
    }
    #[test]
    fn two_top_bottom_and_left_right() {
        let l = layout(mask(2), SplitOpts::default());
        assert_eq!(l[0].unwrap().h, 0.5);
        assert_eq!(l[1].unwrap().y, 0.5);
        let l = layout(
            mask(2),
            SplitOpts {
                two: TwoSplit::LeftRight,
                ..Default::default()
            },
        );
        assert_eq!(l[1].unwrap().x, 0.5);
    }
    #[test]
    fn quadrants_are_stable() {
        // slot 2 keeps the bottom-left quadrant whether or not slot 1 is present
        let a = layout([true, true, true, false], SplitOpts::default());
        let b = layout([true, true, true, true], SplitOpts::default());
        assert_eq!(a[2], b[2]);
        assert_eq!(a[3], None);
    }
    #[test]
    fn all_arrangements_tile_the_window() {
        for arr in Arrange::ALL {
            for n in 3..=4 {
                let l = layout(
                    mask(n),
                    SplitOpts {
                        multi: arr,
                        ..Default::default()
                    },
                );
                let mut area = 0.0;
                for r in l.iter().flatten() {
                    area += r.w * r.h;
                    assert!(r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= 1.001 && r.y + r.h <= 1.001);
                }
                if arr != Arrange::Grid {
                    assert!((area - 1.0).abs() < 1e-3, "{arr:?} n={n} area={area}");
                }
            }
        }
    }
    #[test]
    fn px_rounding_covers_window() {
        let (x, y, w, h) = FRect::new(0.5, 0.5, 0.5, 0.5).to_px(1001, 701);
        assert_eq!(x + w, 1001);
        assert_eq!(y + h, 701);
    }
}
