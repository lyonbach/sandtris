use raylib::prelude::*;


#[derive(Debug, Clone, Copy)]
pub struct Grain
{
    pub color: Color,
    pub full: bool
}

impl Grain
{
    pub fn new(color: Color, full: bool) -> Grain {
        Grain { color: color, full: full }
    }
}
pub enum ShapeType
{
    L,
    S,
    I,
    O
}

pub type SandGrid = Vec<Vec<Grain>>;
