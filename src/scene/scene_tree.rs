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
    // Create a new node without hierarchy info.
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

    // Add hierarchy info.
    pub fn add_child(&mut self, parent: NodeId, new_node: NodeId) {
        match self.nodes[parent.index].last_child {
            // If parent has no child.
            None => {
                self.nodes[parent.index].first_child = Some(new_node);
            }
            // If parent has child.
            Some(last_child) => {
                // Make the parent's last child and the new node siblings.
                self.nodes[last_child.index].next_sibling = Some(new_node);
                self.nodes[new_node.index].previous_sibling = Some(last_child);
            }
        }

        self.nodes[parent.index].last_child = Some(new_node);
        self.nodes[new_node.index].parent = Some(parent);
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
