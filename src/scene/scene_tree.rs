pub struct Arena<T> {
    nodes: Vec<Node<T>>,
}

pub struct Node<T> {
    parent: Option<NodeId>,
    previous_sibling: Option<NodeId>,
    next_sibling: Option<NodeId>,
    first_child: Option<NodeId>,
    last_child: Option<NodeId>,

    /// The actual data which will be stored within the tree.
    pub data: T,
}

#[derive(Copy, Clone)]
pub struct NodeId {
    index: usize,
}

impl<T> Arena<T> {
    pub fn new_node(&mut self, data: T) -> NodeId {
        // Get the next free index.
        let next_index = self.nodes.len();

        // Push the node into the arena.
        self.nodes.push(Node {
            parent: None,
            first_child: None,
            last_child: None,
            previous_sibling: None,
            next_sibling: None,
            data,
        });

        // Return the node identifier.
        NodeId { index: next_index }
    }

    pub fn traverse_children(&mut self, parent: NodeId) {
        let mut current = parent;
        let mut just_went_upward = false;
        loop {
            let node = &self.nodes[current.index];
            if node.first_child.is_some() {
                current = node.first_child.unwrap().clone();
                just_went_upward = false;
            } else if node.next_sibling.is_some() {
                current = node.next_sibling.unwrap().clone();
                just_went_upward = false;
            } else if node.parent.is_some() {
                current = node.parent.unwrap().clone();
                just_went_upward = true;
            } else {
                break;
            }

            log::info!("Current node: {}", current.index);
        }
    }
}
