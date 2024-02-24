use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphNode {
    pub name: String,
    pub children: Vec<GraphNode>,
    pub value: usize,
}
