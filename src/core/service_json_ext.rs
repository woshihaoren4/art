use std::future::Future;
use std::marker::PhantomData;
use serde::{Serialize};
use serde::de::DeserializeOwned;
use wd_tools::{PFErr, PFOk};
use crate::core::{Ctx, JsonInput, Output, Service, ServiceEntity};

#[async_trait::async_trait]
pub trait JsonServiceExt<In:Serialize+DeserializeOwned+Default+Send+'static,Out:Serialize+Send+'static>:Send {
    async fn input(&self,ctx: Ctx, se:&mut ServiceEntity)->anyhow::Result<In>{
        let res = se.transform_config(|c:Option<JsonInput>|{
            c
        });
        match res {
            Some(s)=>{
                s.default_transform(ctx).await
            }
            None =>{
                Err(anyhow::anyhow!("JsonServiceExt:{}.{} ServiceEntity config must json Value",se.service_name,se.node_name))
            }
        }

    }
    async fn output(&self,out:Out)->anyhow::Result<Output>{
        match serde_json::to_value(out){
            Ok(o)=>Output::new(o).ok(),
            Err(e)=>anyhow::anyhow!("JsonServiceExt:output failed:{e}").err()
        }
    }
    async fn call(&self, ctx: Ctx, input:In,se:ServiceEntity) -> anyhow::Result<Out>;
}

#[async_trait::async_trait]
impl<Fut,In,Out,F> JsonServiceExt<In,Out> for F
where F:Fn(Ctx,In,ServiceEntity)->Fut + Send+Sync+'static,
    Fut:Future<Output =anyhow::Result<Out>>+Send,
    In: Serialize+DeserializeOwned+Send+Sync+'static+Default,
    Out: Serialize+Send+Sync+'static,
{
    async fn call(&self, ctx: Ctx, input: In, se: ServiceEntity) -> anyhow::Result<Out> {
        self(ctx,input,se).await
    }
}

pub struct JsonService<T,In,Out>{
    inner:T,
    _in:PhantomData<In>,
    _out:PhantomData<Out>,
}
impl<T,In,Out> JsonService<T,In,Out>
where T:JsonServiceExt<In,Out>+ Sync + 'static,
      In: Serialize+DeserializeOwned+Send+Sync+Default+'static,
      Out: Serialize+Send+Sync+'static,
{
    pub fn new(inner:T)->Self{
        Self{inner,_in:PhantomData::default(),_out:PhantomData::default()}
    }
}

#[async_trait::async_trait]
impl<T,In,Out> Service for JsonService<T,In,Out>
where T: JsonServiceExt<In,Out> + Sync + 'static,
    In: Serialize+DeserializeOwned+Send+Sync+Default+'static,
    Out: Serialize+Send+Sync+'static,
{
    async fn call(&self, ctx: Ctx, mut node: ServiceEntity) -> anyhow::Result<Output> {
        let input = self.inner.input(ctx.clone(),&mut node).await?;
        let output = JsonServiceExt::call(&self.inner,ctx,input,node).await?;
        self.inner.output(output).await
    }
}

#[cfg(test)]
mod test{
    use serde_json::Value;
    use crate::core::{Ctx, MapServiceLoader, ServiceEntity};
    use crate::core::service_json_ext::JsonService;

    #[tokio::test]
    async fn test_json_service_ext(){
        let _map = MapServiceLoader::default()
            .register_service("",JsonService::new(|_c:Ctx,_i:Value,_se:ServiceEntity|async {
                Ok(Value::default())
            }));
    }
}