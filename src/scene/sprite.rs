use crate::resource::{texture, mesh, material};
use mesh::{Mesh};
use material::Material2d;

pub struct Sprite2d {
    pub mesh: Mesh,
    pub material: Material2d,
}
