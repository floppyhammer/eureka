use crate::render::render_graph::config::{
    AntiAliasingMode, RenderGraphConfig,
};
use crate::render::render_graph::RenderGraph;
use std::collections::{BTreeMap, HashMap};
use winit::event::WindowEvent;
use winit::window::Window;

pub struct EguiMain {
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub show_config_inspector: bool,
    pub show_node_graph: bool,
    pub node_positions: HashMap<String, egui::Pos2>,
    pub selected_node: Option<String>,
    pub config: RenderGraphConfig,
    pub pending_textures_delta: egui::TexturesDelta,
    pub canvas_pan: egui::Vec2,
    pub canvas_zoom: f32,
    pub auto_layout_done: bool,
    pub has_3d: bool,
}

impl EguiMain {
    pub fn new(window: &Window) -> Self {
        let egui_ctx = egui::Context::default();
        let viewport_id = egui_ctx.viewport_id();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            viewport_id,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let config = RenderGraphConfig::default();

        Self {
            egui_ctx,
            egui_state,
            show_config_inspector: true,
            show_node_graph: false,
            node_positions: HashMap::new(),
            selected_node: None,
            config,
            pending_textures_delta: egui::TexturesDelta::default(),
            canvas_pan: egui::vec2(30.0, 30.0),
            canvas_zoom: 1.0,
            auto_layout_done: false,
            has_3d: true,
        }
    }

    pub fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) -> egui_winit::EventResponse {
        self.egui_state.on_window_event(window, event)
    }

    pub fn run(
        &mut self,
        window: &Window,
        config: &mut RenderGraphConfig,
    ) -> (Vec<egui::ClippedPrimitive>, egui::TexturesDelta, f32) {
        let raw_input = self.egui_state.take_egui_input(window);
        let mut config_changed = false;
        let temp_graph = RenderGraph::build_from_config_and_camera(config.clone(), self.has_3d);
        let ctx = self.egui_ctx.clone();

        self.egui_ctx.begin_pass(raw_input);
        self.ui(&ctx, config, &temp_graph, &mut config_changed);
        let full_output = self.egui_ctx.end_pass();

        self.egui_state.handle_platform_output(window, full_output.platform_output);

        let pixels_per_point = full_output.pixels_per_point;
        let tris = self.egui_ctx.tessellate(full_output.shapes, pixels_per_point);

        self.pending_textures_delta.set.extend(full_output.textures_delta.set);
        self.pending_textures_delta.free.extend(full_output.textures_delta.free);

        let delta_to_send = std::mem::take(&mut self.pending_textures_delta);

        (tris, delta_to_send, pixels_per_point)
    }

    pub fn recover_textures_delta(&mut self, unsent: egui::TexturesDelta) {
        let mut combined_set = unsent.set;
        combined_set.extend(self.pending_textures_delta.set.drain(..));
        self.pending_textures_delta.set = combined_set;

        let mut combined_free = unsent.free;
        combined_free.extend(self.pending_textures_delta.free.drain(..));
        self.pending_textures_delta.free = combined_free;
    }

    pub fn auto_layout(&mut self, render_graph: &RenderGraph) {
        let topo = render_graph.topological_sort();
        let deps = render_graph.dependencies();

        let mut depths: HashMap<String, usize> = HashMap::new();
        for node in &topo {
            let depth = if let Some(dep_nodes) = deps.get(node) {
                dep_nodes
                    .iter()
                    .filter_map(|d| depths.get(d))
                    .max()
                    .map(|&max_d| max_d + 1)
                    .unwrap_or(0)
            } else {
                0
            };
            depths.insert(node.clone(), depth);
        }

        for name in render_graph.node_names() {
            depths.entry(name.to_string()).or_insert(0);
        }

        let mut columns: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        for node in &topo {
            let d = depths.get(node).copied().unwrap_or(0);
            columns.entry(d).or_default().push(node.clone());
        }
        for name in render_graph.node_names() {
            if !topo.contains(&name.to_string()) {
                columns.entry(0).or_default().push(name.to_string());
            }
        }

        let col_width = 240.0;
        let row_height = 80.0;
        let start_x = 40.0;
        let start_y = 40.0;

        self.node_positions.clear();
        for (&col, nodes) in &columns {
            let x = start_x + col as f32 * col_width;
            for (row, node) in nodes.iter().enumerate() {
                let y = start_y + row as f32 * row_height;
                self.node_positions.insert(node.clone(), egui::pos2(x, y));
            }
        }
        self.canvas_pan = egui::vec2(20.0, 20.0);
        self.canvas_zoom = 1.0;
        self.auto_layout_done = true;
    }

    fn ui(
        &mut self,
        ctx: &egui::Context,
        config: &mut RenderGraphConfig,
        render_graph: &RenderGraph,
        config_changed: &mut bool,
    ) {
        egui::Window::new("Eureka Viewport Controls")
            .show(ctx, |ui| {
                ui.checkbox(&mut self.show_config_inspector, "Render Config Inspector");
                ui.checkbox(&mut self.show_node_graph, "Render Graph Node Topology");
            });

        // 1. Config Inspector Window
        if self.show_config_inspector {
            egui::Window::new("⚙️ Render Graph Config Inspector")
                .open(&mut self.show_config_inspector)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Presets");
                        ui.horizontal(|ui| {
                            if ui.button("Low").clicked() {
                                *config = RenderGraphConfig::low();
                                *config_changed = true;
                            }
                            if ui.button("Medium").clicked() {
                                *config = RenderGraphConfig::medium();
                                *config_changed = true;
                            }
                            if ui.button("High").clicked() {
                                *config = RenderGraphConfig::high();
                                *config_changed = true;
                            }
                            if ui.button("Ultra").clicked() {
                                *config = RenderGraphConfig::ultra();
                                *config_changed = true;
                            }
                        });
                        ui.separator();

                        ui.heading("Shadows");
                        if ui.checkbox(&mut config.shadow.enabled, "Enabled").changed() { *config_changed = true; }
                        if ui.add(egui::Slider::new(&mut config.shadow.map_resolution, 512..=4096).text("Resolution")).changed() { *config_changed = true; }
                        if ui.add(egui::Slider::new(&mut config.shadow.cascade_count, 1..=8).text("Cascades")).changed() { *config_changed = true; }
                        ui.separator();

                        ui.heading("SSAO");
                        if ui.checkbox(&mut config.ssao.enabled, "Enabled").changed() { *config_changed = true; }
                        if ui.add(egui::Slider::new(&mut config.ssao.sample_count, 8..=128).text("Samples")).changed() { *config_changed = true; }
                        if ui.add(egui::Slider::new(&mut config.ssao.radius, 0.1..=5.0).text("Radius")).changed() { *config_changed = true; }
                        ui.separator();

                        ui.heading("SSGI");
                        if ui.checkbox(&mut config.ssgi.enabled, "Enabled").changed() { *config_changed = true; }
                        if ui.add(egui::Slider::new(&mut config.ssgi.ray_steps, 8..=128).text("Ray Steps")).changed() { *config_changed = true; }
                        ui.separator();

                        ui.heading("SSR");
                        if ui.checkbox(&mut config.ssr.enabled, "Enabled").changed() { *config_changed = true; }
                        if ui.add(egui::Slider::new(&mut config.ssr.max_steps, 16..=256).text("Max Steps")).changed() { *config_changed = true; }
                        ui.separator();

                        ui.heading("Volumetric Lighting");
                        if ui.checkbox(&mut config.volumetric.enabled, "Enabled").changed() { *config_changed = true; }
                        if ui.add(egui::Slider::new(&mut config.volumetric.sample_count, 8..=128).text("Samples")).changed() { *config_changed = true; }
                        ui.separator();

                        ui.heading("Bloom");
                        if ui.checkbox(&mut config.bloom.enabled, "Enabled").changed() { *config_changed = true; }
                        if ui.add(egui::Slider::new(&mut config.bloom.intensity, 0.0..=5.0).text("Intensity")).changed() { *config_changed = true; }
                        if ui.add(egui::Slider::new(&mut config.bloom.threshold, 0.0..=5.0).text("Threshold")).changed() { *config_changed = true; }
                        ui.separator();

                        ui.heading("Anti-Aliasing");
                        egui::ComboBox::from_label("AA Mode")
                            .selected_text(format!("{:?}", config.aa_mode))
                            .show_ui(ui, |ui| {
                                if ui.selectable_value(&mut config.aa_mode, AntiAliasingMode::None, "None").changed() { *config_changed = true; }
                                if ui.selectable_value(&mut config.aa_mode, AntiAliasingMode::Fxaa, "FXAA").changed() { *config_changed = true; }
                                if ui.selectable_value(&mut config.aa_mode, AntiAliasingMode::Taa, "TAA").changed() { *config_changed = true; }
                                if ui.selectable_value(&mut config.aa_mode, AntiAliasingMode::TaaAndFxaa, "TAA + FXAA").changed() { *config_changed = true; }
                            });
                        ui.separator();

                        ui.heading("Tonemapping");
                        if ui.checkbox(&mut config.tonemapping.enabled, "Enabled").changed() { *config_changed = true; }
                    });
                });
        }

        // 2. Interactive Node Graph Window
        if self.show_node_graph {
            let mut show_node_graph = self.show_node_graph;
            egui::Window::new("📊 Render Graph Node Topology (Interactive)")
                .open(&mut show_node_graph)
                .default_size(egui::vec2(850.0, 500.0))
                .show(ctx, |ui| {
                    if !self.auto_layout_done {
                        self.auto_layout(render_graph);
                    }

                    let node_names = render_graph.node_names();
                    let dependencies = render_graph.dependencies();
                    let topo_order = render_graph.topological_sort();
                    let execution_steps: HashMap<String, usize> = topo_order
                        .iter()
                        .enumerate()
                        .map(|(i, name)| (name.clone(), i + 1))
                        .collect();

                    // Toolbar
                    ui.horizontal(|ui| {
                        if ui.button("🔄 Auto Layout").on_hover_text("Rearrange nodes by topological execution depth").clicked() {
                            self.auto_layout(render_graph);
                        }
                        if ui.button("🔍 Reset View").clicked() {
                            self.canvas_pan = egui::vec2(20.0, 20.0);
                            self.canvas_zoom = 1.0;
                        }
                        if ui.button("➕").on_hover_text("Zoom in").clicked() {
                            self.canvas_zoom = (self.canvas_zoom * 1.15).min(2.5);
                        }
                        if ui.button("➖").on_hover_text("Zoom out").clicked() {
                            self.canvas_zoom = (self.canvas_zoom / 1.15).max(0.4);
                        }
                        ui.separator();
                        let total_edges: usize = dependencies.values().map(|v| v.len()).sum();
                        ui.label(format!("Nodes: {} | Edges: {} | Zoom: {:.0}%", node_names.len(), total_edges, self.canvas_zoom * 100.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.weak("💡 Pan: Drag background | Zoom: Mouse wheel");
                        });
                    });
                    ui.separator();

                    // Canvas Area
                    let available_size = ui.available_size();
                    let (response, painter) = ui.allocate_painter(available_size, egui::Sense::click_and_drag());
                    let canvas_rect = response.rect;

                    // Mouse wheel zoom - smooth & proportional (smooth_scroll_delta is in pixels)
                    if response.hovered() {
                        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
                        if scroll.abs() > 0.5 {
                            // ~100 px scroll -> ~26% zoom change (e^0.003*100 ≈ 1.35)
                            let factor = (scroll * 0.003_f32).exp();
                            self.canvas_zoom = (self.canvas_zoom * factor).clamp(0.4, 2.5);
                        }
                    }

                    let mut any_node_dragged = false;
                    let mut hovered_node: Option<String> = None;

                    // Helper coordinate transforms
                    let zoom = self.canvas_zoom;
                    let pan = self.canvas_pan;
                    let to_screen = |pos: egui::Pos2| -> egui::Pos2 {
                        canvas_rect.min + pan + pos.to_vec2() * zoom
                    };

                    let node_w = 160.0 * zoom;
                    let node_h = 50.0 * zoom;

                    // Ensure all nodes have positions
                    for name in &node_names {
                        self.node_positions.entry((*name).to_string()).or_insert_with(|| {
                            egui::pos2(50.0, 50.0)
                        });
                    }

                    // Calculate screen rects for all nodes
                    let mut node_rects: HashMap<String, egui::Rect> = HashMap::new();
                    for name in &node_names {
                        if let Some(&pos) = self.node_positions.get(*name) {
                            let screen_pos = to_screen(pos);
                            let rect = egui::Rect::from_min_size(screen_pos, egui::vec2(node_w, node_h));
                            node_rects.insert((*name).to_string(), rect);
                        }
                    }

                    // Pre-check hovering
                    if let Some(ptr_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                        for name in &node_names {
                            if let Some(&rect) = node_rects.get(*name) {
                                if rect.contains(ptr_pos) {
                                    hovered_node = Some((*name).to_string());
                                }
                            }
                        }
                    }

                    // 1. Draw Edges (Smooth Cubic Bézier Curves with directional indicators)
                    for (to_node, from_nodes) in dependencies {
                        let to_rect = match node_rects.get(to_node) {
                            Some(r) => *r,
                            None => continue,
                        };
                        for from_node in from_nodes {
                            let from_rect = match node_rects.get(from_node) {
                                Some(r) => *r,
                                None => continue,
                            };

                            let p1 = from_rect.right_center();
                            let p2 = to_rect.left_center();

                            let is_related = self.selected_node.as_deref() == Some(from_node.as_str())
                                || self.selected_node.as_deref() == Some(to_node.as_str())
                                || hovered_node.as_deref() == Some(from_node.as_str())
                                || hovered_node.as_deref() == Some(to_node.as_str());

                            let stroke = if is_related {
                                egui::Stroke::new(2.5 * zoom.max(0.7), egui::Color32::from_rgb(255, 205, 70))
                            } else if self.selected_node.is_some() || hovered_node.is_some() {
                                egui::Stroke::new(1.0 * zoom.max(0.5), egui::Color32::from_rgba_unmultiplied(70, 95, 125, 60))
                            } else {
                                egui::Stroke::new(1.5 * zoom.max(0.6), egui::Color32::from_rgb(90, 135, 185))
                            };

                            // Cubic Bézier calculation
                            let dx = (p2.x - p1.x).abs().max(40.0 * zoom) * 0.5;
                            let c1 = p1 + egui::vec2(dx, 0.0);
                            let c2 = p2 - egui::vec2(dx, 0.0);

                            let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                                [p1, c1, c2, p2],
                                false,
                                egui::Color32::TRANSPARENT,
                                stroke,
                            );
                            painter.add(bezier);

                            // Connector circles
                            painter.circle_filled(p1, 3.5 * zoom.max(0.6), stroke.color);
                            painter.circle_filled(p2, 3.5 * zoom.max(0.6), stroke.color);

                            // Directional arrow at destination port
                            let arrow_len = 8.0 * zoom.max(0.6);
                            let arrow_w = 4.0 * zoom.max(0.6);
                            let a1 = p2 - egui::vec2(arrow_len, -arrow_w);
                            let a2 = p2 - egui::vec2(arrow_len, arrow_w);
                            painter.line_segment([a1, p2], stroke);
                            painter.line_segment([a2, p2], stroke);
                        }
                    }

                    // 2. Draw Nodes
                    for name in &node_names {
                        let pos = self.node_positions.get_mut(*name).unwrap();
                        let screen_pos = to_screen(*pos);
                        let node_rect = egui::Rect::from_min_size(screen_pos, egui::vec2(node_w, node_h));

                        let id = ui.make_persistent_id(name);
                        let drag_response = ui.interact(node_rect, id, egui::Sense::click_and_drag());
                        if drag_response.dragged() {
                            *pos += drag_response.drag_delta() / zoom;
                            any_node_dragged = true;
                        }

                        if drag_response.clicked() {
                            if self.selected_node.as_deref() == Some(*name) {
                                self.selected_node = None;
                            } else {
                                self.selected_node = Some((*name).to_string());
                            }
                        }

                        let is_selected = self.selected_node.as_deref() == Some(*name);
                        let is_hovered = drag_response.hovered();

                        let (cat_name, badge_bg, badge_fg) = node_visual_info(name);

                        let border_stroke = if is_selected {
                            egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 205, 70))
                        } else if is_hovered {
                            egui::Stroke::new(1.8, egui::Color32::from_rgb(130, 190, 255))
                        } else {
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 75, 95))
                        };

                        let bg_color = if is_selected {
                            egui::Color32::from_rgb(45, 52, 65)
                        } else if is_hovered {
                            egui::Color32::from_rgb(38, 44, 56)
                        } else {
                            egui::Color32::from_rgb(26, 30, 38)
                        };

                        let corner_radius = 6.0 * zoom.max(0.6);
                        let header_h = 17.0 * zoom;
                        let header_rect = egui::Rect::from_min_size(node_rect.min, egui::vec2(node_w, header_h));

                        // Card background
                        painter.rect(
                            node_rect,
                            corner_radius,
                            bg_color,
                            border_stroke,
                            egui::StrokeKind::Inside,
                        );

                        // Header strip
                        painter.rect_filled(
                            header_rect,
                            egui::CornerRadius {
                                nw: corner_radius as u8,
                                ne: corner_radius as u8,
                                sw: 0,
                                se: 0,
                            },
                            badge_bg,
                        );

                        // Category badge
                        painter.text(
                            header_rect.left_center() + egui::vec2(8.0 * zoom, 0.0),
                            egui::Align2::LEFT_CENTER,
                            cat_name,
                            egui::FontId::proportional((9.5 * zoom).max(8.0)),
                            badge_fg,
                        );

                        // Step number
                        if let Some(step) = execution_steps.get(*name) {
                            painter.text(
                                header_rect.right_center() - egui::vec2(8.0 * zoom, 0.0),
                                egui::Align2::RIGHT_CENTER,
                                format!("#{}", step),
                                egui::FontId::monospace((9.5 * zoom).max(8.0)),
                                egui::Color32::from_rgb(175, 185, 200),
                            );
                        }

                        // Node Title
                        let body_center = egui::pos2(
                            node_rect.center().x,
                            node_rect.min.y + header_h + (node_h - header_h) * 0.5,
                        );
                        painter.text(
                            body_center,
                            egui::Align2::CENTER_CENTER,
                            *name,
                            egui::FontId::proportional((12.0 * zoom).max(9.0)),
                            egui::Color32::WHITE,
                        );
                    }

                    // If empty background was dragged, pan the canvas
                    if response.dragged() && !any_node_dragged {
                        self.canvas_pan += response.drag_delta();
                    }

                    // 3. Bottom Detail Panel for Selected Node
                    if let Some(selected) = self.selected_node.clone() {
                        let incoming: Vec<String> = dependencies.get(&selected).cloned().unwrap_or_default();
                        let outgoing: Vec<String> = dependencies
                            .iter()
                            .filter_map(|(to_node, from_nodes)| {
                                if from_nodes.contains(&selected) {
                                    Some(to_node.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let step_str = execution_steps
                            .get(&selected)
                            .map(|s| format!("Step #{}", s))
                            .unwrap_or_else(|| "Unknown Step".to_string());

                        let mut deselect = false;
                        egui::Area::new(egui::Id::new("node_detail_popup"))
                            .fixed_pos(canvas_rect.left_bottom() + egui::vec2(10.0, -90.0))
                            .show(ctx, |ui| {
                                egui::Frame::popup(ui.style()).fill(egui::Color32::from_rgba_unmultiplied(20, 24, 30, 240)).show(ui, |ui| {
                                    ui.set_max_width(450.0);
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&selected).strong().color(egui::Color32::GOLD));
                                        ui.weak(format!("({})", step_str));
                                        if ui.small_button("✖").clicked() {
                                            deselect = true;
                                        }
                                    });
                                    ui.small(format!("Inputs ({}): {}", incoming.len(), incoming.join(", ")));
                                    ui.small(format!("Outputs ({}): {}", outgoing.len(), outgoing.join(", ")));
                                });
                            });
                        if deselect {
                            self.selected_node = None;
                        }
                    }
                });
            self.show_node_graph = show_node_graph;
        }
    }
}

fn node_visual_info(name: &str) -> (&'static str, egui::Color32, egui::Color32) {
    if name.starts_with("prepare") {
        ("PREP", egui::Color32::from_rgb(24, 52, 64), egui::Color32::from_rgb(90, 190, 225))
    } else if name.contains("cull") {
        ("CULL", egui::Color32::from_rgb(52, 34, 68), egui::Color32::from_rgb(180, 130, 230))
    } else if name == "shadow" {
        ("SHADOW", egui::Color32::from_rgb(68, 42, 20), egui::Color32::from_rgb(240, 150, 70))
    } else if name == "prepass" || name == "depth_pyramid" {
        ("DEPTH", egui::Color32::from_rgb(20, 58, 62), egui::Color32::from_rgb(70, 210, 210))
    } else if name.contains("ssao") || name.contains("ssgi") || name.contains("ssr") || name.contains("volumetric") {
        ("LIGHT/FX", egui::Color32::from_rgb(64, 30, 64), egui::Color32::from_rgb(220, 110, 220))
    } else if name == "taa" || name == "fxaa" || name == "bloom" || name.contains("tone") {
        ("POST", egui::Color32::from_rgb(24, 60, 36), egui::Color32::from_rgb(90, 210, 130))
    } else {
        ("DRAW", egui::Color32::from_rgb(28, 45, 72), egui::Color32::from_rgb(110, 160, 240))
    }
}

pub struct EguiBackendRenderer {
    pub renderer: egui_wgpu::Renderer,
    pub pending_set_textures: Vec<(egui::TextureId, egui::epaint::ImageDelta)>,
    pub pending_free_textures: Vec<egui::TextureId>,
}

impl EguiBackendRenderer {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let renderer = egui_wgpu::Renderer::new(device, output_format, Default::default());
        Self {
            renderer,
            pending_set_textures: Vec::new(),
            pending_free_textures: Vec::new(),
        }
    }

    pub fn receive_textures_delta(&mut self, textures_delta: egui::TexturesDelta) {
        self.pending_set_textures.extend(textures_delta.set);
        self.pending_free_textures.extend(textures_delta.free);
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        screen_size: (u32, u32),
        scale_factor: f32,
        tris: &[egui::ClippedPrimitive],
    ) -> Vec<wgpu::CommandBuffer> {
        for id in self.pending_free_textures.drain(..) {
            self.renderer.free_texture(&id);
        }

        for (id, image_delta) in self.pending_set_textures.drain(..) {
            self.renderer.update_texture(device, queue, id, &image_delta);
        }

        if tris.is_empty() {
            return Vec::new();
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [screen_size.0, screen_size.1],
            pixels_per_point: scale_factor,
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Egui Command Encoder"),
        });

        let mut command_buffers = self.renderer.update_buffers(device, queue, &mut encoder, tris, &screen_descriptor);

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let rpass_static: &mut wgpu::RenderPass<'static> = unsafe {
                std::mem::transmute(&mut rpass as &mut wgpu::RenderPass<'_>)
            };
            self.renderer.render(rpass_static, tris, &screen_descriptor);
        }

        command_buffers.push(encoder.finish());
        command_buffers
    }
}
