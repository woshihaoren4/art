use wd_tools::PFErr;
use crate::core::{Ctx, ServiceEntity};

pub enum NextPlan {
    Nodes(Vec<ServiceEntity>),
    End,
    Wait,
}

pub trait Plan: Send {
    fn string(&self) -> String {
        "".into()
    }
    fn start_node_name(&self) -> &str {
        "start"
    }
    fn end_node_name(&self) -> &str {
        "end"
    }
    fn get(&mut self, name: &str) -> Option<ServiceEntity>;
    fn next(&mut self, ctx: Ctx, name: &str) -> anyhow::Result<NextPlan>;
}

impl Plan for (){
    fn get(&mut self, _name: &str) -> Option<ServiceEntity> {
        None
    }

    fn next(&mut self, _ctx: Ctx, _name: &str) -> anyhow::Result<NextPlan> {
        anyhow::anyhow!("this is empty plan!!!").err()
    }
}