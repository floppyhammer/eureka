use crate::render::{Mesh, RenderContext, TextureId};

#[derive(Copy, Clone)]
pub struct ExtractedSky {
    pub texture: TextureId,
}

#[derive(Clone)]
pub(crate) struct SkyImportedResources {
    pub texture: Option<TextureId>,
    pub mesh: Option<Mesh>,
}

impl SkyImportedResources {
    pub(crate) fn new() -> Self {
        Self {
            texture: None,
            mesh: None,
        }
    }
}

pub(crate) fn prepare_sky(
    imported_resources: &mut SkyImportedResources,
    render_server: &RenderContext,
    texture_id: &TextureId,
    mesh_allocator: &mut crate::render::mesh_allocator::MeshAllocator,
) {
    if imported_resources.mesh.is_none() {
        imported_resources.mesh = Some(Mesh::default_skybox(&render_server.queue, mesh_allocator));
    }

    if imported_resources.texture.is_none() || imported_resources.texture.unwrap() != *texture_id {
        imported_resources.texture = Some(*texture_id);
    }
}
