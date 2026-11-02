//! Piece movement rules: `Movement` struct and `HookType` enum.

/// Movement rules for a piece type.
#[derive(Clone, Debug)]
pub struct Movement {
    pub slides: Vec<(u8, u8)>, // (direction, max_range) 0=unlimited
    pub jumps: Vec<(i8, i8)>,  // (delta_row, delta_col)
    pub hook: Option<HookType>,
    pub area: u8,               // 0=none, 2=lion, 3=lion_dog
    pub range_capture: Vec<u8>, // directions for range capture
    pub igui: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum HookType {
    Orthogonal,
    Diagonal,
}

impl Movement {
    pub fn empty() -> Self {
        Movement {
            slides: Vec::new(),
            jumps: Vec::new(),
            hook: None,
            area: 0,
            range_capture: Vec::new(),
            igui: false,
        }
    }
}