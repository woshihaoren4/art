use crate::core::{Ctx, JsonService, JsonServiceExt, Output, ServiceEntity};
use std::collections::HashMap;
use std::sync::Arc;
use wd_tools::PFArc;

#[async_trait::async_trait]
pub trait Service: Send {
    async fn call(&self, ctx: Ctx, node: ServiceEntity) -> anyhow::Result<Output>;
}

pub struct EmptyServiceImpl;

#[async_trait::async_trait]
impl Service for EmptyServiceImpl {
    async fn call(&self, _ctx: Ctx, _node: ServiceEntity) -> anyhow::Result<Output> {
        Ok(Output::default())
    }
}

#[async_trait::async_trait]
pub trait ServiceLoader: Send {
    // async fn insert(&mut self,_name:&str,_service:Arc<dyn Service + Sync + 'static>){}
    // async fn remove(&mut self, _name: &str) -> Option<Arc<dyn Service + Sync + 'static>>{None}
    async fn load(&self, name: &str) -> Option<Arc<dyn Service + Sync + 'static>>;
}

#[derive(Default)]
pub struct MapServiceLoader {
    pub map: HashMap<String, Arc<dyn Service + Sync + 'static>>,
}
impl MapServiceLoader {
    pub fn register_service<N: Into<String>, S: Service + Sync + 'static>(
        mut self,
        name: N,
        service: S,
    ) -> Self {
        self.map.insert(name.into(), Arc::new(service));
        self
    }
    pub fn register_json_ext_service<N: Into<String>,T,In,Out>(mut self,name:N,service:T)-> Self
    where T: JsonServiceExt<In,Out> + Sync + 'static,
          In: serde::Serialize+serde::de::DeserializeOwned+Send+Sync+Default+'static,
          Out: serde::Serialize+Send+Sync+'static,
    {
        self.map.insert(name.into(),JsonService::new(service).arc());self
    }
}
#[async_trait::async_trait]
impl ServiceLoader for MapServiceLoader {
    async fn load(&self, name: &str) -> Option<Arc<dyn Service + Sync + 'static>> {
        self.map.get(name).map(|x| x.clone())
    }
}
