use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphNode {
    name: String,
    children: Vec<GraphNode>,
    value: i32,
}
