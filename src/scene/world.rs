use std::thread::current;
use cgmath::Point2;
use indextree::{Arena, Descendants, NodeEdge, NodeId};
use crate::render::gizmo::Gizmo;
use crate::resource::RenderServer;
use crate::scene::{AsNode, Camera2d, CameraInfo, Camera3dUniform, NodeType, Camera3d};
use crate::server::{InputEvent, InputServer};
use crate::Singletons;

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
}

impl World {
    pub fn new() -> Self {
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
        }
    }

    pub fn add_node(&mut self, mut new_node: Box<dyn AsNode>, parent: Option<NodeId>) -> NodeId {
        log::info!("Added node: {}", new_node.node_type().to_string());

        // FIXME: Should move this after adding node in the tree.
        new_node.ready();

        let node_type = new_node.node_type();

        let id = self.arena.new_node(new_node);

        match node_type {
            NodeType::Camera2d => { self.current_camera2d = Some(id); }
            NodeType::Camera3d => { self.current_camera3d = Some(id); }
            NodeType::Light => { self.lights.push(id); }
            _ => {}
        }

        if self.root_node.is_none() {
            self.root_node = Some(id);
        } else {
            // Set the root as the parent if there's none.
            let parent = match parent {
                Some(p) => p,
                None => self.root_node.unwrap(),
            };

            parent.append(id, &mut self.arena);
        }

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

                // for id in iter {
                //     ids.push(id);
                // }
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

    pub fn update(
        &mut self,
        dt: f32,
        singletons: &mut Singletons,
    ) {
        match self.arena[self.current_camera2d.unwrap()].get().as_any().downcast_ref::<Camera2d>() {
            Some(camera) => {
                self.camera_info.view_size = camera.view_size;
                self.camera_info.position = camera.transform.position;
            }
            None => panic!("Camera isn't a Camera2d!"),
        }
        match self.arena[self.current_camera3d.unwrap()].get().as_any().downcast_ref::<Camera3d>() {
            Some(camera) => {
                self.camera_info.bind_group = Some(camera.bind_group.clone());
            }
            None => panic!("Camera isn't a Camera3d!"),
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
        // match self.arena[self.current_camera2d.unwrap()].get().as_any().downcast_mut::<Camera2d>() {
        //     Some(camera) => {
        //         camera.when_view_size_changes(new_width, new_height);
        //     }
        //     None => panic!("Camera isn't a Camera2d!"),
        // }
        // match &mut self.arena[self.current_camera3d.unwrap()].get().as_any().downcast_mut::<Camera3d>() {
        //     Some(camera) => {
        //         camera.when_view_size_changes(new_width, new_height);
        //     }
        //     None => panic!("Camera isn't a Camera3d!"),
        // }
    }
}
