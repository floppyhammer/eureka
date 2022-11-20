use indextree::{Arena, Descendants, NodeEdge, NodeId};
use crate::render::gizmo::Gizmo;
use crate::resource::RenderServer;
use crate::scene::AsNode;
use crate::server::InputEvent;
use crate::Singletons;

pub struct World {
    // Type Box<dyn AsNode> is a trait object;
    // it’s a stand-in for any type inside a Box that implements the AsNode trait.
    pub arena: Arena<Box<dyn AsNode>>,

    root_node: Option<NodeId>,

    gizmo: Gizmo,
}

impl World {
    pub fn new() -> Self {
        let mut arena = Arena::new();

        let gizmo = Gizmo::new();

        Self {
            arena,
            root_node: None,
            gizmo,
        }
    }

    pub fn add_node(&mut self, mut new_node: Box<dyn AsNode>, parent: Option<NodeId>) -> NodeId {
        log::info!("Added node: {}", new_node.node_type().to_string());

        // FIXME: Should move this after adding node in the tree.
        new_node.ready();

        let id = self.arena.new_node(new_node);

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

    pub fn input(&mut self, input_event: &InputEvent) {
        // Input events propagate reversely.
        for id in self.traverse().iter().rev() {
            self.arena[*id].get_mut().input(input_event);
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        singletons: Option<&Singletons>,
    ) {
        for id in self.traverse() {
            self.arena[id]
                .get_mut()
                .update(dt, singletons);
        }
    }

    pub fn draw<'a, 'b: 'a>(
        &'b mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        for id in self.traverse() {
            self.arena[id]
                .get()
                .draw(render_pass, render_server, singletons);
        }

        self.gizmo.draw(render_pass, render_server, singletons);
    }
}
