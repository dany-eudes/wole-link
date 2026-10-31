//! The menu's icons: thin strokes on a 16 × 16 grid centred on (0, 0), in the manner of Windows 11
//! and GNOME, turned into polygons for `Canvas::fill` so they stay sharp at any scale.
//!
//! Every stroke is a closed shape of its own: segments are quads, joints and ends are discs, a
//! circle or ellipse outline is an outer ring minus its inner one. `fill` uses the nonzero rule,
//! so with every shape wound the same way (holes the other way) overlapping strokes merge into one
//! coverage instead of darkening where they cross.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Language.
    Globe,
    /// About.
    Info,
    /// The website.
    External,
    /// Google Play.
    Play,
    /// Buy Me a Coffee.
    Coffee,
    /// Contact.
    Mail,
    /// The Linux applications menu.
    Apps,
}

/// Stroke width, in grid units (logical pixels).
const STROKE: f32 = 1.4;

type Point = (f32, f32);

/// The icon's shapes, in grid units around (0, 0).
pub fn polygons(icon: Icon) -> Vec<Vec<Point>> {
    let mut pen = Pen::default();
    match icon {
        Icon::Globe => {
            pen.ellipse((0.0, 0.0), 6.8, 6.8);
            pen.ellipse((0.0, 0.0), 2.9, 6.8);
            pen.line(&[(-6.8, 0.0), (6.8, 0.0)]);
        }
        Icon::Info => {
            pen.ellipse((0.0, 0.0), 6.8, 6.8);
            pen.dot((0.0, -3.3), 1.0);
            pen.line(&[(0.0, -0.6), (0.0, 3.6)]);
        }
        Icon::External => {
            pen.line(&[(-1.5, -5.5), (-6.0, -5.5), (-6.0, 6.0), (5.5, 6.0), (5.5, 1.5)]);
            pen.line(&[(-0.5, 0.5), (6.5, -6.5)]);
            pen.line(&[(1.5, -6.5), (6.5, -6.5), (6.5, -1.5)]);
        }
        Icon::Play => pen.closed(&[(-4.5, -6.5), (-4.5, 6.5), (6.5, 0.0)]),
        Icon::Coffee => {
            pen.closed(&[(-6.5, -2.0), (3.5, -2.0), (3.5, 3.0), (1.5, 6.0), (-4.5, 6.0), (-6.5, 3.0)]);
            pen.line(&[(3.5, -0.5), (5.5, -0.5), (6.8, 0.8), (6.8, 1.7), (5.5, 3.0), (3.5, 3.0)]);
            pen.line(&[(-3.5, -7.0), (-3.5, -4.5)]);
            pen.line(&[(0.5, -7.0), (0.5, -4.5)]);
        }
        Icon::Mail => {
            pen.closed(&[(-7.0, -5.0), (7.0, -5.0), (7.0, 5.0), (-7.0, 5.0)]);
            pen.line(&[(-7.0, -5.0), (0.0, 1.0), (7.0, -5.0)]);
        }
        Icon::Apps => {
            for (x, y) in [(-6.0, -6.0), (1.5, -6.0), (-6.0, 1.5), (1.5, 1.5)] {
                pen.closed(&[(x, y), (x + 4.5, y), (x + 4.5, y + 4.5), (x, y + 4.5)]);
            }
        }
    }
    pen.shapes
}

#[derive(Default)]
struct Pen {
    shapes: Vec<Vec<Point>>,
}

impl Pen {
    /// An open stroke through `points`, with round joins and ends.
    fn line(&mut self, points: &[Point]) {
        for pair in points.windows(2) {
            self.segment(pair[0], pair[1]);
        }
        for &p in points {
            self.dot(p, STROKE / 2.0);
        }
    }

    /// A stroke through `points` and back to the first.
    fn closed(&mut self, points: &[Point]) {
        let mut ring = points.to_vec();
        ring.push(points[0]);
        self.line(&ring);
    }

    fn segment(&mut self, (ax, ay): Point, (bx, by): Point) {
        let len = (bx - ax).hypot(by - ay);
        if len == 0.0 {
            return;
        }
        let (nx, ny) = (-(by - ay) / len * STROKE / 2.0, (bx - ax) / len * STROKE / 2.0);
        self.add(vec![(ax + nx, ay + ny), (bx + nx, by + ny), (bx - nx, by - ny), (ax - nx, ay - ny)], true);
    }

    fn dot(&mut self, center: Point, r: f32) {
        self.add(oval(center, r, r, 16), true);
    }

    /// The outline of an ellipse, centred on its path.
    fn ellipse(&mut self, center: Point, rx: f32, ry: f32) {
        let h = STROKE / 2.0;
        self.add(oval(center, rx + h, ry + h, 48), true);
        self.add(oval(center, rx - h, ry - h, 48), false);
    }

    /// Winds `shape` one way for filled shapes and the other for holes.
    fn add(&mut self, mut shape: Vec<Point>, filled: bool) {
        if (area(&shape) > 0.0) != filled {
            shape.reverse();
        }
        self.shapes.push(shape);
    }
}

fn oval((cx, cy): Point, rx: f32, ry: f32, steps: usize) -> Vec<Point> {
    (0..steps)
        .map(|i| {
            let t = i as f32 / steps as f32 * std::f32::consts::TAU;
            (cx + rx * t.cos(), cy + ry * t.sin())
        })
        .collect()
}

/// Twice the signed area: the sign tells the winding.
fn area(shape: &[Point]) -> f32 {
    (0..shape.len())
        .map(|i| {
            let ((ax, ay), (bx, by)) = (shape[i], shape[(i + 1) % shape.len()]);
            ax * by - bx * ay
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Icon; 7] =
        [Icon::Globe, Icon::Info, Icon::External, Icon::Play, Icon::Coffee, Icon::Mail, Icon::Apps];

    #[test]
    fn every_icon_fits_its_grid_and_winds_one_way() {
        for icon in ALL {
            let shapes = polygons(icon);
            assert!(!shapes.is_empty(), "{icon:?}");
            for shape in &shapes {
                for &(x, y) in shape {
                    assert!(x.abs() <= 8.0 && y.abs() <= 8.0, "{icon:?} reaches ({x}, {y})");
                }
            }
            // Holes only come with the outline they're cut from.
            let holes = shapes.iter().filter(|s| area(s) < 0.0).count();
            assert!(holes < shapes.len(), "{icon:?}");
        }
    }
}
