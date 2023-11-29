use gfx_maths::Vec2;
use crate::helpers::distance2d;

// Event handler trait
pub trait EventHandler {
    fn handle_event(&self, event: &str);
}

// Vector to store individual animation graph nodes
// 2D position of the animation graph
#[derive(Clone, Debug)]
pub struct AnimGraph {
    pub nodes: Vec<AnimGraphNode>,
    pub position: Vec2,
}

// Individual nodes
#[derive(Clone, Debug)]
pub struct AnimGraphNode {
    pub name: String,
    pub position: Vec2,
    pub animation: String,
    pub events: Vec<String>, // Vector to store events associated with the node
}

// Constructor method to create a new animation graph
impl AnimGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            position: Vec2::new(0.0, 0.0),
        }
    }

    // Method to add a node to the animation graph
    pub fn add_node(&mut self, name: String, position: Vec2, animation: String) -> Result<(), String> {
        // Check if a node with the same name already exists
        if self.nodes.iter().any(|node| node.name == name) {
            return Err(format!("Node with name '{}' already exists.", name));
        }

        self.nodes.push(AnimGraphNode {
            name,
            position,
            animation,
            events: Vec::new(), // Initialize events vector for the new node
        });

        Ok(())
    }

    // Method to calculate and retrieve the weights of nodes based on their influence
    pub fn weights(&self) -> Vec<(String, f64)> {
        let mut weights = Vec::new();
        let position = self.position;

        // Node weights are 0 if there is no influence, and 1 if there is full influence.
        // Nodes beyond a distance of 2.0 units have no impact.
        // If our position coincides exactly with a node, we receive a weight of 1.0 with no consideration for other nodes.
        let mut max_weight = 0.0;

        for node in &self.nodes {
            let distance = distance2d(position, node.position) as f64;
            let weight = 1.0 - (distance / 2.0).min(1.0);

            // Add node and corresponding weight to the weights vector if the weight is greater than 0
            if weight > 0.0 {
                weights.push((node.animation.clone(), weight));
            }

            // Update max_weight if the current weight is greater
            if weight > max_weight {
                max_weight = weight;
            }

            // If weight is 1.0, only one node can have full influence, so retain that node and break
            if weight == 1.0 {
                weights.retain(|(_, w)| *w == max_weight);
                break;
            }
        }

        weights
    }
}

impl EventHandler for AnimGraph {
    fn handle_event(&self, event: &str) {
        for node in &self.nodes {
            // Process each node's events
            if node.events.contains(&event.to_string()) {
                println!("Handling event '{}' for node: {}", event, node.name);
            }
        }
    }
}
