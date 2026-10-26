use crate::{
    assets::{MaterialRepository, MeshRepository},
    render::context::{RenderContext, RenderContextConfig},
    scene::Scene,
};

pub struct Renderer {
    context: RenderContext,
}

impl Renderer {
    /// # Errors
    pub fn new(context_config: &RenderContextConfig) -> anyhow::Result<Self> {
        let context = RenderContext::new(context_config)?;
        Ok(Self { context })
    }

    /// # Errors
    pub fn render(
        &self,
        scene: &Scene,
        mesh_repo: &MeshRepository,
        material_repo: &MaterialRepository,
    ) -> anyhow::Result<()> {
        let draw_commands = scene.extract_draw_list();
        Ok(())
    }

    #[must_use]
    pub const fn context(&self) -> &RenderContext {
        &self.context
    }
}
