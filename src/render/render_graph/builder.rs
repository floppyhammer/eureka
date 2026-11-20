use crate::render::render_graph::config::{AntiAliasingMode, RenderGraphConfig};
use crate::render::render_graph::graph::RenderGraph;
use crate::render::render_graph::nodes::{
    BloomNode, ClearNode, CullingNode, DepthPyramidNode, FxaaNode, IBLNode, IndirectApplyNode,
    LightCullingNode, MeshNode, PrePassNode, PrepareInstancesNode, PrepareMaterialsNode,
    PrepareViewNode, ShadowNode, SkyboxNode, SpriteNode, SsaoNode, SsgiNode, SsrNode, TaaNode,
    ToneMappingNode, TransparentMeshNode, VolumetricApplyNode, VolumetricNode,
};

/// 基于 RenderGraphConfig 动态组装并连接渲染图的 Builder。
pub struct RenderGraphBuilder {
    config: RenderGraphConfig,
    has_3d: bool,
}

impl RenderGraphBuilder {
    pub fn new(config: RenderGraphConfig) -> Self {
        Self {
            config,
            has_3d: true,
        }
    }

    pub fn with_3d(mut self, has_3d: bool) -> Self {
        self.has_3d = has_3d;
        self
    }

    /// 根据配置动态构建包含节点与依赖边的 RenderGraph。
    pub fn build(self) -> RenderGraph {
        let mut graph = RenderGraph::new();

        if !self.has_3d {
            // 2D-only 渲染图构建 (纯 2D 极简渲染图，用于 3D 引擎中的 2D 文字与图片 UI)
            graph.add_node("prepare_view", PrepareViewNode::default());
            graph.add_node("prepare_materials", PrepareMaterialsNode::default());
            graph.add_node("clear", ClearNode);
            graph.add_node("sprite", SpriteNode::default());

            graph.add_node_edge("prepare_view", "sprite");
            graph.add_node_edge("prepare_materials", "sprite");
            graph.add_node_edge("clear", "sprite");

            return graph;
        }

        // -----------------------------------------------------------------
        // 1. 基础必备节点 (Base Core Nodes)
        // -----------------------------------------------------------------
        graph.add_node("prepare_view", PrepareViewNode::default());
        graph.add_node("prepare_materials", PrepareMaterialsNode::default());
        graph.add_node("prepare_instances", PrepareInstancesNode::default());
        graph.add_node("culling", CullingNode::default());
        graph.add_node("prepass", PrePassNode::default());
        graph.add_node("depth_pyramid", DepthPyramidNode::default());
        graph.add_node("ibl", IBLNode::default());
        graph.add_node("clear", ClearNode);
        graph.add_node("skybox", SkyboxNode::default());
        graph.add_node("mesh", MeshNode::default());
        graph.add_node("transparent_mesh", TransparentMeshNode::default());
        graph.add_node("light_culling", LightCullingNode::default());
        graph.add_node("sprite", SpriteNode::default());

        // -----------------------------------------------------------------
        // 2. 根据配置条件注册可选特效节点与边 (Conditional Feature Nodes)
        // -----------------------------------------------------------------
        if self.config.shadow.enabled {
            graph.add_node("shadow", ShadowNode::default());
        }

        if self.config.volumetric.enabled {
            graph.add_node("volumetric", VolumetricNode::default());
            graph.add_node("volumetric_apply", VolumetricApplyNode::default());
        }

        if self.config.ssao.enabled {
            graph.add_node("ssao", SsaoNode::default());
        }

        // -----------------------------------------------------------------
        // 3. 动态构建基础依赖边 (Dynamic Base Dependency Wiring)
        // -----------------------------------------------------------------
        graph.add_node_edge("prepare_view", "culling");
        graph.add_node_edge("prepare_instances", "culling");
        graph.add_node_edge("culling", "prepass");
        graph.add_node_edge("prepass", "depth_pyramid");

        graph.add_node_edge("culling", "mesh");
        graph.add_node_edge("prepass", "mesh");
        graph.add_node_edge("prepare_view", "skybox");
        graph.add_node_edge("prepare_view", "light_culling");
        graph.add_node_edge("ibl", "mesh");
        graph.add_node_edge("ibl", "transparent_mesh");
        graph.add_node_edge("clear", "skybox");
        graph.add_node_edge("skybox", "mesh");
        graph.add_node_edge("light_culling", "mesh");

        if self.config.shadow.enabled {
            graph.add_node_edge("shadow", "mesh");
        }

        if self.config.ssao.enabled {
            graph.add_node_edge("culling", "ssao");
            graph.add_node_edge("prepass", "ssao");
            graph.add_node_edge("ssao", "mesh");
        }

        if self.config.volumetric.enabled {
            graph.add_node_edge("prepare_view", "volumetric");
            graph.add_node_edge("light_culling", "volumetric");
            if self.config.shadow.enabled {
                graph.add_node_edge("shadow", "volumetric");
            }
            graph.add_node_edge("volumetric", "volumetric_apply");
            graph.add_node_edge("mesh", "volumetric_apply");
            graph.add_node_edge("volumetric_apply", "transparent_mesh");
        } else {
            graph.add_node_edge("mesh", "transparent_mesh");
        }

        // -----------------------------------------------------------------
        // 4. 动态槽路由与后处理链条拼接 (Dynamic Slot Routing & Post-FX Chain)
        // -----------------------------------------------------------------
        use crate::render::render_graph::standard_resources;

        let mut current_color = standard_resources::main_color();
        let mut last_stage = if self.config.volumetric.enabled {
            "volumetric_apply".to_string()
        } else {
            "transparent_mesh".to_string()
        };

        // 4.1 TAA 节点
        let has_taa = matches!(
            self.config.aa_mode,
            AntiAliasingMode::Taa | AntiAliasingMode::TaaAndFxaa
        );
        if has_taa {
            let next_color = standard_resources::taa_output();
            graph.add_node(
                "taa",
                TaaNode::with_slots(current_color.clone(), next_color.clone()),
            );
            graph.add_node_edge("prepass", "taa");
            graph.add_node_edge(&last_stage, "taa");
            current_color = next_color;
            last_stage = "taa".to_string();
        }

        // 4.2 SSGI & SSR 节点
        if self.config.ssgi.enabled {
            graph.add_node(
                "ssgi",
                SsgiNode::with_slots(current_color.clone(), standard_resources::ssgi_output()),
            );
            graph.add_node_edge("prepass", "ssgi");
            graph.add_node_edge(&last_stage, "ssgi");
        }

        if self.config.ssr.enabled {
            graph.add_node(
                "ssr",
                SsrNode::with_slots(current_color.clone(), standard_resources::ssr_output()),
            );
            graph.add_node_edge("prepass", "ssr");
            graph.add_node_edge(&last_stage, "ssr");
            if self.config.ssao.enabled {
                graph.add_node_edge("ssao", "ssr");
            }
        }

        let has_indirect = self.config.ssgi.enabled || self.config.ssr.enabled;
        if has_indirect {
            let next_color = standard_resources::ssr_combined();
            graph.add_node(
                "indirect_apply",
                IndirectApplyNode::with_slots(current_color.clone(), next_color.clone()),
            );
            graph.add_node_edge(&last_stage, "indirect_apply");
            if self.config.ssgi.enabled {
                graph.add_node_edge("ssgi", "indirect_apply");
            }
            if self.config.ssr.enabled {
                graph.add_node_edge("ssr", "indirect_apply");
            }
            current_color = next_color;
            last_stage = "indirect_apply".to_string();
        }

        // 4.3 Bloom 节点
        let bloom_resource = if self.config.bloom.enabled {
            let bloom_out = standard_resources::bloom_texture();
            graph.add_node(
                "bloom",
                BloomNode::with_slots(current_color.clone(), bloom_out.clone()),
            );
            graph.add_node_edge(&last_stage, "bloom");
            Some(bloom_out)
        } else {
            None
        };

        // 4.4 ToneMapping 节点
        let has_fxaa = matches!(
            self.config.aa_mode,
            AntiAliasingMode::Fxaa | AntiAliasingMode::TaaAndFxaa
        );
        if self.config.tonemapping.enabled {
            let next_color = if has_fxaa {
                standard_resources::hdr_resolved()
            } else {
                standard_resources::final_output()
            };
            graph.add_node(
                "tonemapping",
                ToneMappingNode::with_slots(
                    current_color.clone(),
                    bloom_resource,
                    next_color.clone(),
                ),
            );
            graph.add_node_edge(&last_stage, "tonemapping");
            if self.config.bloom.enabled {
                graph.add_node_edge("bloom", "tonemapping");
            }
            current_color = next_color;
            last_stage = "tonemapping".to_string();
        }

        // 4.5 FXAA 节点
        if has_fxaa {
            let next_color = standard_resources::final_output();
            graph.add_node(
                "fxaa",
                FxaaNode::with_slots(current_color.clone(), next_color.clone()),
            );
            graph.add_node_edge(&last_stage, "fxaa");
            last_stage = "fxaa".to_string();
        }

        // 2D Sprite UI 最终叠加
        graph.add_node_edge("prepare_view", "sprite");
        graph.add_node_edge("prepare_materials", "sprite");
        graph.add_node_edge(&last_stage, "sprite");

        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_graph_builder_presets() {
        let presets = [
            RenderGraphConfig::low(),
            RenderGraphConfig::medium(),
            RenderGraphConfig::high(),
            RenderGraphConfig::ultra(),
        ];

        for config in presets {
            let graph = RenderGraphBuilder::new(config).build();
            let order = graph.topological_sort();
            assert!(!order.is_empty(), "Execution order should not be empty");
            assert!(order.contains(&"sprite".to_string()));
            assert!(order.contains(&"mesh".to_string()));
        }
    }

    #[test]
    fn test_render_graph_builder_2d_only() {
        let config = RenderGraphConfig::default();
        let graph = RenderGraphBuilder::new(config).with_3d(false).build();
        let order = graph.topological_sort();
        assert!(!order.is_empty(), "2D execution order should not be empty");
        assert!(order.contains(&"sprite".to_string()));
        assert!(order.contains(&"prepare_view".to_string()));
        assert!(order.contains(&"clear".to_string()));
        assert!(!order.contains(&"mesh".to_string()), "2D-only graph should not contain 3D mesh node");
        assert!(!order.contains(&"shadow".to_string()), "2D-only graph should not contain shadow node");
    }
}
