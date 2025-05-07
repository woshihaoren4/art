use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::core::ServiceEntityJson;

#[derive(Default,Debug,Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphNode {
    pub node_name: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub service: Option<ServiceEntityJson>,
}

#[derive(Default,Debug,Clone, Serialize, Deserialize)]
pub struct Graph {
    pub start: String,
    pub end: String,
    pub node_set: HashMap<String, GraphNode>,
}
