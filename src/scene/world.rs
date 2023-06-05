use crate::render::gizmo::Gizmo;
use crate::render::RenderServer;
use crate::scene::{AsNode, Camera2d, Camera3d, CameraUniform, CameraInfo, NodeType};
use crate::window::{InputEvent, InputServer};
use crate::Singletons;
use cgmath::Point2;
use indextree::{Arena, Descendants, NodeEdge, NodeId};
use std::thread::current;

pub struct World {
    // Type Box<dyn AsNode> is a trait object;
    // it’s a stand-in for any type inside a Box that implements the AsNode trait.
    pub arena: Arena<Box<dyn AsNode>>,

    root_node: Option<NodeId>,

    current_camera2d: Option<NodeId>,
    current_camera3d: Option<NodeId>,
    lights: Vec<NodeId>,

    camera_info: CameraInfo,

    gizmo: Gizmo,

    view_size: Point2<u32>,
}

impl World {
    pub fn new(view_size: Point2<u32>) -> Self {
        let mut arena = Arena::new();

        let gizmo = Gizmo::new();

        Self {
            arena,
            root_node: None,
            current_camera2d: None,
            current_camera3d: None,
            lights: vec![],
            gizmo,
            camera_info: CameraInfo::default(),
            view_size,
        }
    }

    pub fn add_node(&mut self, mut new_node: Box<dyn AsNode>, parent: Option<NodeId>) -> NodeId {
        log::info!("Added node: {}", new_node.node_type().to_string());

        let node_type = new_node.node_type();

        // Create a new node.
        let id = self.arena.new_node(new_node);

        // Handle some special nodes.
        match node_type {
            NodeType::Camera2d => {
                self.current_camera2d = Some(id);
            }
            NodeType::Camera3d => {
                self.current_camera3d = Some(id);
            }
            NodeType::Light => {
                self.lights.push(id);
            }
            _ => {}
        }

        if self.root_node.is_none() {
            self.root_node = Some(id);
        } else {
            // Set the root as the parent if no parent is provided.
            let parent = match parent {
                Some(p) => p,
                None => self.root_node.unwrap(),
            };

            parent.append(id, &mut self.arena);
        }

        // After the node is added to the tree, call its ready() function.
        self.arena[id].get_mut().ready();

        id
    }

    fn traverse(&self) -> Vec<NodeId> {
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

                ids = iter.map(|id| id).collect();
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
        match node_ptr.as_any().downcast_ref::<T>() {
            Some(node_ptr) => Some(node_ptr),
            None => None,
        }
    }

    /// Get a mutable reference to a node by its ID.
    pub fn get_node_mut<T: 'static>(&mut self, id: NodeId) -> Option<&mut T> {
        // Get the pointer to the node.
        let node_ptr = self.arena[id].get_mut();

        // Downcast it to the original type.
        match node_ptr.as_any_mut().downcast_mut::<T>() {
            Some(node_ptr) => Some(node_ptr),
            None => None,
        }
    }

    pub fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        if let Some(node_id) = self.current_camera2d {
            // Get camera info.
            if let Some(camera2d) = self.get_node::<Camera2d>(node_id) {
                self.camera_info.position = camera2d.transform.position;
                self.camera_info.view_size = self.view_size;
            }
        }

        if let Some(node_id) = self.current_camera3d {
            if let Some(camera3d) = self.get_node::<Camera3d>(node_id) {
                self.camera_info.bind_group = Some(camera3d.bind_group.clone());
            }
        }

        for id in self.traverse() {
            self.arena[id]
                .get_mut()
                .update(dt, &self.camera_info, singletons);
        }
    }

    pub fn draw<'a, 'b: 'a>(
        &'b mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        singletons: &'b Singletons,
    ) {
        for id in self.traverse() {
            self.arena[id]
                .get()
                .draw(render_pass, &self.camera_info, singletons);
        }

        self.gizmo.draw(render_pass, &self.camera_info, singletons);
    }

    pub fn when_view_size_changes(&mut self, new_size: Point2<u32>) {
        self.view_size = new_size;

        if let Some(node_id) = self.current_camera2d {
            self.get_node_mut::<Camera2d>(node_id)
                .unwrap()
                .when_view_size_changes(new_size);
        }

        if let Some(node_id) = self.current_camera3d {
            self.get_node_mut::<Camera3d>(node_id)
                .unwrap()
                .when_view_size_changes(new_size);
        }
    }
}
