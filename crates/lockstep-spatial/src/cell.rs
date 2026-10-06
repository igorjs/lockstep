use serde::{Deserialize, Serialize};

/// A cell as one integer: `y * width + x`. The width belongs to the map, not to the cell.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct Cell(pub u32);
