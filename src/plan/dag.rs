use crate::core::{Ctx, NextPlan, Plan, ServiceEntity, ServiceEntityJson};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::mem::take;
use wd_tools::PFErr;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DAGNode {
    pub node_name: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub service: Option<ServiceEntityJson>,
}
impl DAGNode {
    #[allow(unused)]
    pub fn try_from_str(s: &str) -> anyhow::Result<Self> {
        let n = serde_json::from_str(s)?;
        Ok(n)
    }
    pub fn new<S: Into<String>>(node_name: S) -> Self {
        Self {
            node_name: node_name.into(),
            ..Default::default()
        }
    }
    pub fn set_node_name<S: Into<String>>(mut self, node_name: S) -> Self {
        let node_name = node_name.into();
        if let Some(ref mut s) = self.service {
            s.node_name = node_name.clone();
        }
        self.node_name = node_name;
        self
    }
    pub fn set_from<T: Into<String>>(mut self, from: Vec<T>) -> Self {
        let from = from.into_iter().map(|x| x.into()).collect::<Vec<String>>();
        self.from = from;
        self
    }
    pub fn add_from<T: Into<String>>(&mut self, node_name: T) {
        let node_name = node_name.into();
        for i in self.from.iter() {
            if i == &node_name {
                return;
            }
        }
        self.from.push(node_name)
    }
    pub fn have_from(&self, f: &str) -> bool {
        for i in self.from.iter() {
            if i == f {
                return true;
            }
        }
        false
    }
    pub fn remove_from_and_take_service(&mut self, f: &str) -> Option<ServiceEntity> {
        let mut index = usize::MAX;
        for (i, k) in self.from.iter().enumerate() {
            if k == f {
                index = i;
                break;
            }
        }
        if index < usize::MAX {
            self.from.remove(index);
        }
        if self.from.is_empty() {
            let sej = self.service.take()?;
            Some(sej.into())
        } else {
            None
        }
    }
    pub fn set_to<T: Into<String>>(mut self, to: Vec<T>) -> Self {
        let to = to.into_iter().map(|x| x.into()).collect::<Vec<String>>();
        self.to = to;
        self
    }
    pub fn add_to<T: Into<String>>(&mut self, node_name: T) {
        let node_name = node_name.into();
        for i in self.to.iter() {
            if i == &node_name {
                return;
            }
        }
        self.to.push(node_name)
    }
    pub fn have_to(&self, t: &str) -> bool {
        for i in self.to.iter() {
            if i == t {
                return true;
            }
        }
        false
    }
    pub fn set_service_entity<E: Into<ServiceEntityJson>>(mut self, service: E) -> Self {
        let se = service.into().set_node_name(self.node_name.to_string());
        self.service = Some(se);
        self
    }
}
impl<N: Into<String>, E: Into<ServiceEntityJson>> From<(N, E)> for DAGNode {
    fn from((n, e): (N, E)) -> Self {
        let n = Self::default().set_node_name(n);
        let e = e.into().set_node_name(n.node_name.clone());
        n.set_service_entity(e)
    }
}
impl From<&str> for DAGNode {
    fn from(value: &str) -> Self {
        serde_json::from_str::<DAGNode>(value).unwrap_or(DAGNode::default())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DAG {
    pub start: String,
    pub end: String,
    pub node_set: HashMap<String, DAGNode>,
}
impl Plan for DAG {
    fn show_plan(&self) -> String {
        serde_json::to_string(self).unwrap_or("".to_string())
    }

    fn start_node_name(&self) -> &str {
        &self.start
    }

    fn end_node_name(&self) -> &str {
        &self.end
    }

    fn get(&mut self, name: &str) -> Option<ServiceEntity> {
        if let Some(n) = self.node_set.get_mut(name) {
            let sej = n.service.take()?;
            Some(ServiceEntity::from(sej))
        } else {
            None
        }
    }

    fn next(&mut self, _ctx: Ctx, name: &str) -> anyhow::Result<NextPlan> {
        if name == self.end {
            return Ok(NextPlan::End);
        }
        let to = if let Some(i) = self.node_set.get_mut(name) {
            take(&mut i.to)
        } else {
            return anyhow::anyhow!("node[{}] not found", name).err();
        };
        let mut next = vec![];
        for i in to {
            if let Some(n) = self.node_set.get_mut(i.as_str()) {
                if let Some(s) = n.remove_from_and_take_service(name) {
                    next.push(s);
                }
            } else {
                return anyhow::anyhow!("node[{}] not found", i).err();
            }
        }
        Ok(NextPlan::Nodes(next))
    }

    fn set_to(&mut self, name: &str, to: Vec<String>) {
        if let Some(s) = self.node_set.get_mut(name) {
            s.to = to;
        }
    }
}
impl DAG {
    pub fn node<Node: Into<DAGNode>>(mut self, node: Node) -> Self {
        let node = node.into();
        self.node_set.insert(node.node_name.clone(), node);
        self
    }
    pub fn nodes<N: Into<DAGNode>, I: IntoIterator<Item = N>>(mut self, nodes: I) -> Self {
        for i in nodes.into_iter() {
            self = self.node(i);
        }
        self
    }
    pub fn edge<F: Into<String>, T: Into<String>>(mut self, from: F, to: T) -> Self {
        let from = from.into();
        let to = to.into();
        //自动追踪起点和终点
        if self.start.is_empty() {
            self.start = from.clone();
        }
        self.end = to.clone();

        if let Some(n) = self.node_set.get_mut(from.as_str()) {
            n.add_to(to.clone());
        } else {
            self.node_set.insert(
                from.clone(),
                DAGNode::default()
                    .set_node_name(from.clone())
                    .set_to(vec![to.clone()]),
            );
        }
        if let Some(n) = self.node_set.get_mut(to.as_str()) {
            n.add_from(from);
        } else {
            self.node_set.insert(
                to.clone(),
                DAGNode::default().set_node_name(to).set_from(vec![from]),
            );
        }
        self
    }
    pub fn edges<F: Into<String>, T: Into<String>, I: IntoIterator<Item = (F, T)>>(
        mut self,
        edges: I,
    ) -> Self {
        for (f, t) in edges {
            self = self.edge(f, t);
        }
        self
    }
    pub fn set_start_node_name<F: Into<String>>(mut self, name: F) -> Self {
        self.start = name.into();
        self
    }
    pub fn set_end_node_name<F: Into<String>>(mut self, name: F) -> Self {
        self.end = name.into();
        self
    }
    pub fn check(self) -> anyhow::Result<Self> {
        //检查起始终止节点
        if self.node_set.get(self.start.as_str()).is_none() {
            return anyhow::anyhow!("not found start node[{}]", self.start).err();
        }
        if self.node_set.get(self.end.as_str()).is_none() {
            return anyhow::anyhow!("not found end node[{}]", self.end).err();
        }
        //检查中间节点
        for (k, v) in self.node_set.iter() {
            if let Some(ref s) = v.service {
                if s.service_name.is_empty() {
                    return anyhow::anyhow!("node[{}].service.name is empty", k).err();
                }
                if s.node_name.is_empty() {
                    return anyhow::anyhow!("node[{}].service.node is empty", k).err();
                }
            } else {
                return anyhow::anyhow!("node[{}].service is empty", k).err();
            }
            if v.node_name == self.start {
                //起始节点
                if !v.from.is_empty() {
                    return anyhow::anyhow!("start node.from must is empty").err();
                }
            } else {
                if v.from.is_empty() {
                    return anyhow::anyhow!("middle node[{}].from must is not empty", v.node_name)
                        .err();
                }
                for i in v.from.iter() {
                    if let Some(n) = self.node_set.get(i) {
                        if !n.have_to(k) {
                            return anyhow::anyhow!("node[{}] <- node[{}] edge not found", k, i)
                                .err();
                        }
                    } else {
                        return anyhow::anyhow!("node[{}] <- node[{}] not found", k, i).err();
                    }
                }
            }
            if v.node_name == self.end {
                //终止节点
                if !v.to.is_empty() {
                    return anyhow::anyhow!("end node.to must is empty").err();
                }
            } else {
                if v.to.is_empty() {
                    return anyhow::anyhow!("start node.to must is not empty").err();
                }
                for i in v.to.iter() {
                    if let Some(n) = self.node_set.get(i) {
                        if !n.have_from(k) {
                            return anyhow::anyhow!("node[{}] -> node[{}] edge not found", k, i)
                                .err();
                        }
                    } else {
                        return anyhow::anyhow!("node[{}] -> node[{}] not found", k, i).err();
                    }
                }
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod test {
    use crate::core::Plan;
    use crate::plan::dag::DAG;

    #[test]
    fn test_dag() {
        let dag = DAG::default()
            .node(("start", ""))
            .node(("A", ""))
            .node(("B", ""))
            .node(("C", ""))
            .nodes([("D", ""), ("E", ""), ("F", "")])
            .nodes([("end", "")])
            .edge("start", "A")
            .edge("start", "B")
            .edge("start", "C")
            .edges([("A", "D"), ("B", "D"), ("C", "D")])
            .edges([("D", "E"), ("D", "F")])
            .edges([("E", "end"), ("F", "end")])
            .check()
            .expect("dag check failed");
        println!(
            "start[{}]->..->end[{}]",
            dag.start_node_name(),
            dag.end_node_name()
        );
        println!("success");
    }
}
