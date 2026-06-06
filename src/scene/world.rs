use crate::core::singleton::Singletons;
use crate::render::draw_command::DrawCommands;
use crate::scene::{AsNode, Camera2d, Camera3d, NodeType};
use crate::window::InputServer;
use crate::animation::property::PropertyChange;
use glam::UVec2;
use indextree::{Arena, NodeEdge, NodeId};
use crate::animation::PropertyProvider;

pub struct World {
    // Type Box<dyn AsNode> is a trait object;
    // it's a stand-in for any type inside a Box that implements the AsNode trait.
    pub arena: Arena<Box<dyn AsNode>>,

    root_node: Option<NodeId>,

    current_camera2d: Option<NodeId>,
    current_camera3d: Option<NodeId>,

    view_size: UVec2,
}

impl World {
    pub fn new(view_size: UVec2) -> Self {
        let arena = Arena::new();

        Self {
            arena,
            root_node: None,
            current_camera2d: None,
            current_camera3d: None,
            view_size,
        }
    }

    pub fn add_node(&mut self, new_node: Box<dyn AsNode>, parent: Option<NodeId>) -> NodeId {
        log::info!("Added node: {}", new_node.node_type().to_string());

        let node_type = new_node.node_type();

        // Create a new arena node.
        let id = self.arena.new_node(new_node);

        // Handle some special nodes.
        match node_type {
            NodeType::Camera2d => {
                self.current_camera2d = Some(id);
            }
            NodeType::Camera3d => {
                self.current_camera3d = Some(id);
            }
            _ => {}
        }

        // Check if this is the first node.
        if self.root_node.is_none() {
            self.root_node = Some(id);
        } else {
            // Set the root as the parent if no parent is provided.
            let parent = parent.unwrap_or_else(|| self.root_node.unwrap());

            parent.append(id, &mut self.arena);
        }

        // After the node is added to the tree, call its ready() function.
        self.arena[id].get_mut().ready();

        id
    }

    pub fn traverse(&self) -> Vec<NodeId> {
        let mut ids: Vec<NodeId> = vec![];

        // Node depth in the tree.
        let mut depths: Vec<i32> = vec![];
        let mut current_depth = 0;

        match self.root_node {
            None => {
                log::warn!("No root node in the scene tree.");
            }
            Some(root) => {
                let iter = root.traverse(&self.arena).filter_map(|edge| match edge {
                    NodeEdge::Start(id) => {
                        depths.push(current_depth);
                        current_depth += 1;

                        Some(id)
                    }
                    NodeEdge::End(_) => {
                        current_depth -= 1;

                        None
                    }
                });

                ids = iter.collect();
            }
        }

        ids
    }

    pub fn input(&mut self, input_server: &mut InputServer) {
        for mut event in input_server.input_events.clone() {
            // Input events propagate reversely.
            for id in self.traverse().iter().rev() {
                self.arena[*id].get_mut().input(&mut event, input_server);
            }
        }
    }

    /// Get a reference to a node by its ID.
    pub fn get_node<T: 'static>(&self, id: NodeId) -> Option<&T> {
        // Get the pointer to the node.
        let node_ptr = self.arena[id].get();

        // Downcast it to the original type.
        node_ptr.as_any().downcast_ref::<T>()
    }

    /// Get a mutable reference to a node by its ID.
    pub fn get_node_mut<T: 'static>(&mut self, id: NodeId) -> Option<&mut T> {
        // Get the pointer to the node.
        let node_ptr = self.arena[id].get_mut();

        // Downcast it to the original type.
        node_ptr.as_any_mut().downcast_mut::<T>()
    }

    pub fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        let ids = self.traverse();
        let mut all_changes: Vec<PropertyChange> = Vec::new();

        for id in ids {
            // 1. Propagate transforms
            let parent_id = self.arena[id].parent();

            // 2D Transform Propagation
            let parent_global_transform2d = parent_id.and_then(|p_id| {
                self.arena[p_id].get().as_node_ui().map(|p| p.get_global_transform())
            });

            if let Some(node2d) = self.arena[id].get_mut().as_node_ui_mut() {
                let local = node2d.get_transform();
                let global = if let Some(parent_global) = parent_global_transform2d {
                    parent_global.combine(&local)
                } else {
                    local
                };
                node2d.set_global_transform(global);
            }

            // 3D Transform Propagation
            let parent_global_transform3d = parent_id.and_then(|p_id| {
                self.arena[p_id].get().as_node_3d().map(|p| p.get_global_transform())
            });

            if let Some(node3d) = self.arena[id].get_mut().as_node_3d_mut() {
                let local = node3d.get_transform();
                let global = if let Some(parent_global) = parent_global_transform3d {
                    parent_global.combine(&local)
                } else {
                    local
                };
                node3d.set_global_transform(global);
            }

            // 2. Node update
            self.arena[id].get_mut().update(dt, singletons);

            // 3. Collect animation changes from AnimationPlayer nodes
            if let Some(player) = self.arena[id].get_mut().as_any_mut().downcast_mut::<crate::animation::player::AnimationPlayer>() {
                all_changes.extend(player.take_changes());
            }
        }

        // 4. Apply all collected property changes
        for change in all_changes {
            if let Some(node) = self.get_node_mut::<Box<dyn AsNode>>(change.target_entity) {
                apply_property_change_to_node(node.as_mut(), &change);
            }
        }

        // Reload assets.
        singletons.asset_server.update();
    }

    pub fn queue_draw(&mut self) -> DrawCommands {
        let mut draw_cmds = DrawCommands::default();
        draw_cmds.view_info.view_size = self.view_size;

        // Collect draw commands from the scene tree.
        for id in self.traverse() {
            self.arena[id].get().draw(&mut draw_cmds);
        }

        draw_cmds
    }

    pub fn when_view_size_changes(&mut self, new_size: UVec2) {
        self.view_size = new_size;

        if let Some(node_id) = self.current_camera2d {
            if let Some(camera) = self.get_node_mut::<Camera2d>(node_id) {
                camera.when_view_size_changes(new_size);
            }
        }

        if let Some(node_id) = self.current_camera3d {
            if let Some(camera) = self.get_node_mut::<Camera3d>(node_id) {
                camera.when_view_size_changes(new_size);
            }
        }
    }
}

fn apply_property_change_to_node(node: &mut dyn AsNode, change: &PropertyChange) {
    // Try to downcast to types that implement PropertyProvider
    if let Some(node3d) = node.as_any_mut().downcast_mut::<crate::scene::d3::node_3d::Node3d>() {
        node3d.set_property(&change.property_path, change.value.clone(), change.weight);
    }
    if let Some(node_ui) = node.as_any_mut().downcast_mut::<crate::scene::d2::node_ui::NodeUi>() {
        node_ui.set_property(&change.property_path, change.value.clone(), change.weight);
    }
}
