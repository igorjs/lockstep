use crate::cell::Cell;
use crate::map::GridMap;
use crate::topology::Topology;

/// Whether `from` can see `to`, with a body whose eyes are `eye_height` steps above the ground.
///
/// The line of sight runs from the eye at `from` down or up to the floor of the cell `to`,
/// interpolated cell by cell along the topology's line. A cell between them blocks the view when its
/// top (its elevation plus its wall height) reaches that line. Because the line starts at an eye and
/// ends at a floor, a wall near the viewer is easier to see over than a wall near the target. Open ground has top equal to its elevation, so a hill blocks.
/// A full wall is 255 steps tall and blocks everything; a low wall blocks only when it reaches the
/// line, so a terrace sees over a garden wall and the ground below does not see up over it. An
/// `eye_height` of zero lets flat ground block, so use at least one. `line` receives the cells
/// from `from` to `to`.
pub fn line_of_sight<T: Topology>(
    map: &GridMap<T>,
    from: Cell,
    to: Cell,
    eye_height: u8,
    line: &mut Vec<Cell>,
) -> bool {
    map.line(from, to, line);
    let steps = line.len() as u32 - 1;
    if steps <= 1 {
        return true;
    }
    let eye_from = map.elevation(from) as u32 + eye_height as u32;
    let floor_to = map.elevation(to) as u32;
    for (index, cell) in line.iter().enumerate().skip(1).take(steps as usize - 1) {
        let index = index as u32;
        let top = map.elevation(*cell) as u32 + map.wall_height(*cell) as u32;
        // Compare in units of 1/steps to avoid dividing: top >= eye_from + (floor_to - eye_from) * index / steps.
        if top * steps >= eye_from * (steps - index) + floor_to * index {
            return false;
        }
    }
    true
}

/// Sight that must work both ways: `a` sees `b` and `b` sees `a`. The line aims from the viewer's eye
/// to the target's floor, so it can clear a wall one way and not the other (a terrace sees down over a
/// garden wall the ground cannot see up over). This is the check for rules that must not depend on
/// who looks.
pub fn line_of_sight_symmetric<T: Topology>(
    map: &GridMap<T>,
    a: Cell,
    b: Cell,
    eye_height: u8,
    line: &mut Vec<Cell>,
) -> bool {
    line_of_sight(map, a, b, eye_height, line) && line_of_sight(map, b, a, eye_height, line)
}
