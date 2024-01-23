use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, ComposerError, NagaModuleDescriptor, ShaderDefValue,
};
use std::borrow::Cow;
use std::collections::HashMap;

pub struct ShaderMaker {
    composer: Composer,
}

impl ShaderMaker {
    pub fn new() -> Self {
        let composer = Composer::default();

        Self { composer }
    }

    /// Add a shader as a composable module so that it can be imported by other shaders.
    pub fn add_composable(&mut self, source: &str, module_name: &str, shader_defs: &[&str]) {
        let module_exists = self.composer.contains_module(module_name);

        if !module_exists {
            let mut shader_defs_map: HashMap<String, ShaderDefValue> = HashMap::new();
            for def in shader_defs.iter() {
                shader_defs_map.insert((*def).into(), Default::default());
            }

            match self
                .composer
                .add_composable_module(ComposableModuleDescriptor {
                    source,
                    shader_defs: shader_defs_map,
                    as_name: Some(module_name.into()),
                    ..Default::default()
                }) {
                Ok(module) => {
                    println!(
                        "Added composable module {} [{:?}]",
                        module.name, module.shader_defs
                    )
                }
                Err(e) => {
                    println!("? -> {e:#?}")
                }
            }
        };
    }

    /// Make a naga module using the shader.
    pub fn make_shader(
        &mut self,
        source: &str,
        shader_defs: &[&str],
    ) -> Option<wgpu::ShaderSource> {
        let mut shader_defs_map: HashMap<String, ShaderDefValue> = HashMap::new();
        for def in shader_defs.iter() {
            shader_defs_map.insert((*def).into(), Default::default());
        }

        match self.composer.make_naga_module(NagaModuleDescriptor {
            source,
            shader_defs: shader_defs_map.into(),
            ..Default::default()
        }) {
            Ok(module) => Some(wgpu::ShaderSource::Naga(Cow::Owned(module))),
            Err(e) => {
                println!("{}", e.emit_to_string(&self.composer));
                None
            }
        }
    }
}
