use crate::core::{Ctx, JsonInput, NextPlan, Plan, ServiceEntity, ServiceEntityJson};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wd_tools::PFErr;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphNode {
    pub in_degree: i32,
    pub node_name: String,
    pub from: Vec<String>,
    pub from_completed: Vec<String>,
    pub to: Vec<String>,
    pub service: ServiceEntityJson,
}
impl GraphNode {
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
        self.service.node_name = node_name.clone();
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
    pub fn from_completed(&mut self, f: &str) -> Option<ServiceEntityJson> {
        if self.from.is_empty() {
            return Some(self.get_service_entity());
        }
        if self.from_completed.is_empty() {
            self.from_completed = self.from.clone();
        }
        let mut index = usize::MAX;
        for (i, k) in self.from_completed.iter().enumerate() {
            if k == f {
                index = i;
                break;
            }
        }
        if index < usize::MAX {
            self.from_completed.remove(index);
        }
        if self.from_completed.is_empty() {
            return Some(self.get_service_entity());
        }
        None
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
        self.service = se;
        self
    }
    pub fn set_service_entity_json<S: Into<String>, J: Into<JsonInput>>(
        self,
        service_name: S,
        input: J,
    ) -> Self {
        self.set_service_entity(
            ServiceEntityJson::default()
                .set_service_name(service_name)
                .set_config(input),
        )
    }
    pub fn get_service_entity(&mut self) -> ServiceEntityJson {
        self.service.clone()
    }
}

impl<N: Into<String>, E: Into<ServiceEntityJson>> From<(N, E)> for GraphNode {
    fn from((n, e): (N, E)) -> Self {
        let n = Self::default().set_node_name(n);
        let e = e.into().set_node_name(n.node_name.clone());
        n.set_service_entity(e)
    }
}
impl From<&str> for GraphNode {
    fn from(value: &str) -> Self {
        GraphNode::try_from_str(value).unwrap_or_else(|e| {
            wd_log::log_field("error", e)
                .field("node", "GraphNode")
                .error("GraphNode from str parse failed");
            GraphNode::default()
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub start: String,
    pub end: String,
    pub node_set: HashMap<String, GraphNode>,
}

impl Graph {
    pub fn get_service_entity(&mut self, node_name: &str) -> Option<ServiceEntityJson> {
        if let Some(n) = self.node_set.get_mut(node_name) {
            Some(n.get_service_entity())
        } else {
            None
        }
    }
    pub fn node<Node: Into<GraphNode>>(mut self, node: Node) -> Self {
        let node = node.into();
        self.node_set.insert(node.node_name.clone(), node);
        self
    }
    pub fn nodes<N: Into<GraphNode>, I: IntoIterator<Item = N>>(mut self, nodes: I) -> Self {
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
                GraphNode::default()
                    .set_node_name(from.clone())
                    .set_to(vec![to.clone()]),
            );
        }
        if let Some(_n) = self.node_set.get_mut(to.as_str()) {
            // n.add_from(from);
        } else {
            self.node_set
                .insert(to.clone(), GraphNode::default().set_node_name(to));
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
    fn update_in_degree(&mut self, start: &str) -> anyhow::Result<()> {
        let to = if let Some(n) = self.node_set.get_mut(start) {
            n.in_degree += 1;
            if n.in_degree > 1 {
                return Ok(());
            }
            if n.to.is_empty() {
                return if n.node_name == self.end {
                    Ok(())
                } else {
                    anyhow::anyhow!("There is an unknown end node[{}]", start).err()
                };
            }
            n.to.clone()
        } else {
            return anyhow::anyhow!("not found node[{}]", start).err();
        };
        for i in to {
            self.update_in_degree(i.as_str())?;
        }
        Ok(())
    }
    fn check_from(&self) -> anyhow::Result<()> {
        for (n, i) in self.node_set.iter() {
            for e in i.from.iter() {
                if let Some(s) = self.node_set.get(e) {
                    if !s.have_to(e) {
                        return anyhow::anyhow!(
                            "node[{n}] prerequisite requirements [{e}], but node[{e}] no to [n]"
                        )
                        .err();
                    }
                }
            }
        }
        Ok(())
    }
    fn check_service(&self) -> anyhow::Result<()> {
        for (n, i) in self.node_set.iter() {
            if i.service.service_name.is_empty() {
                return anyhow::anyhow!("{}.service[{}] not defined", n, i.service.service_name)
                    .err();
            }
            if i.service.node_name.is_empty() {
                return anyhow::anyhow!("{}.node_name[{}] not defined", n, i.service.node_name)
                    .err();
            }
        }
        Ok(())
    }
    pub fn check(mut self) -> anyhow::Result<Self> {
        //检查起始终止节点
        if self.node_set.get(self.start.as_str()).is_none() {
            return anyhow::anyhow!("not found start node[{}]", self.start).err();
        }
        if self.node_set.get(self.end.as_str()).is_none() {
            return anyhow::anyhow!("not found end node[{}]", self.end).err();
        }
        //检查入度，并保证没有未完结的节点
        self.update_in_degree(self.start.clone().as_str())?;
        //检查from，保证前向节点匹配
        self.check_from()?;
        //检查service
        self.check_service()?;

        Ok(self)
    }
}

impl Plan for Graph {
    fn show_plan(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or("{}".into())
    }

    fn start_node_name(&self) -> &str {
        self.start.as_str()
    }

    fn end_node_name(&self) -> &str {
        self.end.as_str()
    }

    fn get(&mut self, name: &str) -> Option<ServiceEntity> {
        self.get_service_entity(name).map(|x| x.into())
    }

    fn next(&mut self, _ctx: Ctx, name: &str) -> anyhow::Result<NextPlan> {
        if name == self.end {
            return Ok(NextPlan::End);
        }
        let to = if let Some(i) = self.node_set.get_mut(name) {
            i.to.clone()
        } else {
            return anyhow::anyhow!("node[{}] not found", name).err();
        };
        let mut next = vec![];
        for i in to {
            if let Some(n) = self.node_set.get_mut(i.as_str()) {
                if let Some(s) = n.from_completed(i.as_str()) {
                    next.push(s.into());
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

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};
    use crate::core::{CtxSerdeExt, EngineRT, JsonInput, Plan};
    use crate::plan::graph::{Graph, GraphNode};
    use crate::service::ext::ServiceLoaderWrap;
    use serde_json::json;

    #[test]
    fn test_graph() {
        let graph = Graph::default()
            .node(("start", r#"{"service_name":"start"}"#))
            .node(("A", r#"{"service_name":"a"}"#))
            .node(("B", r#"{"service_name":"b"}"#))
            .node(("C", r#"{"service_name":"c"}"#))
            .nodes([
                ("D", r#"{"service_name":"d"}"#),
                ("E", r#"{"service_name":"e"}"#),
                ("F", r#"{"service_name":"f"}"#),
            ])
            .nodes([("end", r#"{"service_name":"end"}"#)])
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
            graph.start_node_name(),
            graph.end_node_name()
        );
        println!("{}", graph.show_plan());
        println!("success");
    }

    #[tokio::test]
    async fn test_select_graph() {
        #[derive(Default, Debug, Clone, Serialize, Deserialize)]
        #[serde(default)]
        struct AddInOut{
            a:isize,
            b:isize,
            result:isize,
        }
        let rt = EngineRT::default()
            .set_service_loader(ServiceLoaderWrap::default()
                .register_json_ext_service("add",|_ctx, mut io:AddInOut, _se|async move{
                    io.result = io.a + io.b;
                    Ok(io)
                }))
            .build();
        let select_cfg = json!({
        "conditions": {
            "cond": {
                "cond": "and",
                "sub": [{
                    "cond": {
                        "cond": "greater",
                        "sub": [{
                            "value": "${{start.number}}"
                        }, {
                            "value": 666
                        }]
                    }
                }]
            }
        },
        "true_to_nodes": ["A"],
        "false_to_nodes": ["B"]
        });
        let a_cfg = json!({
            "a":"${{start.number}}",
            "b":1,
        });
        let b_cfg = json!({
            "a":"${{start.number}}",
            "b":-1,
        });

        let plan = Graph::default()
            .node(("start",r#"{"service_name":"start","config":{"transform_rule":{"number":{"quote":"number"}}}}"#))
            .node(GraphNode::new("select").set_service_entity_json("flow_select",JsonInput::default().set_default_json(select_cfg)))
            .node(GraphNode::new("A").set_service_entity_json("add",JsonInput::default().set_default_json(a_cfg)))
            .node(GraphNode::new("B").set_service_entity_json("add",JsonInput::default().set_default_json(b_cfg)))
            .node(("end", r#"{"service_name":"end","config":{"transform_rule":{"result":{"quote":"number"}}}}"#))
            .edges([("start","select"),("select","A"),("select","B"),("A","end"),("B","end")])
            .check()
            .unwrap();
    }
}
