use std::sync::Arc;
use crate::core::{Ctx, JsonService, JsonServiceExt, MapServiceLoader, Service, ServiceEntity, ServiceLoader};
use crate::service;
use crate::service::agent::Workflow;

pub struct ServiceLoaderWrap{
    pub map_loader : MapServiceLoader,
    pub service_loader : Arc<dyn ServiceLoader + Sync + 'static>,
}
impl ServiceLoaderWrap{
    pub fn new()->Self{
        Self{
            map_loader: MapServiceLoader::default(),
            service_loader: Arc::new(()),
        }
    }
    pub fn set_map_loader(mut self,msl:MapServiceLoader)->Self{
        self.map_loader = msl;self
    }
    pub fn set_service_loader<T:ServiceLoader + Sync  + 'static>(mut self,loader:T)->Self{
        self.service_loader = Arc::new(loader);self
    }
    pub fn register_service<N: Into<String>, S: Service + Sync + 'static>(
        mut self,
        name: N,
        service: S,
    ) -> Self {
        self.map_loader = self.map_loader.register_service(name, service);self
    }
    pub fn register_json_ext_service<N: Into<String>, T, In, Out>(
        mut self,
        name: N,
        service: T,
    ) -> Self
    where
        T: JsonServiceExt<In, Out> + Sync + 'static,
        In: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + Default + 'static,
        Out: serde::Serialize + Send + Sync + 'static,
    {
        self.map_loader = self.map_loader.register_json_ext_service(name, service);self
    }
}
impl Default for ServiceLoaderWrap {
    fn default() -> Self {
        Self::new()
            .set_map_loader(MapServiceLoader::default()
                .register_json_ext_service("start", service::custom::Start {})
                .register_json_ext_service("end", service::custom::End {})
                .register_json_ext_service("workflow",Workflow::new()))

    }
}

#[async_trait::async_trait]
impl ServiceLoader for ServiceLoaderWrap {
    async fn load(&self, name: &str) -> Option<Arc<dyn Service + Sync + 'static>> {
        let res = self.map_loader.load(name).await;
        if res.is_some(){
            return res
        }
        self.service_loader.load(name).await
    }
}