use std::future::Future;
use std::marker::PhantomData;
use serde::{Serialize};
use serde::de::DeserializeOwned;
use serde_json::Value;
use wd_tools::{PFErr, PFOk};
use crate::core::{Ctx, Output, Service, ServiceEntity};

#[async_trait::async_trait]
pub trait JsonServiceExt<In:DeserializeOwned,Out:Serialize>:Send {
    fn input(&self,_ctx: Ctx, se:&mut ServiceEntity)->anyhow::Result<In>{
        let res = se.transform_config(|c:Option<Value>|{
            if let Some(s) = c {
                Some(serde_json::from_value::<In>(s))
            }else{
                None
            }
        });
        match res {
            Some(Ok(o))=>Ok(o),
            Some(Err(e))=>{
                Err(anyhow::anyhow!("JsonServiceExt:{}.{} ServiceEntity transform input failed:{}",se.service_name,se.node_name,e))
            }
            None=>{
                Err(anyhow::anyhow!("JsonServiceExt:{}.{} ServiceEntity config must json Value",se.service_name,se.node_name))
            }
        }
    }
    fn output(&self,out:Out)->anyhow::Result<Output>{
        match serde_json::to_value(out){
            Ok(o)=>Output::new(o).ok(),
            Err(e)=>anyhow::anyhow!("JsonServiceExt:output failed:{e}").err()
        }
    }
    async fn call(&self, ctx: Ctx, input:In,se:ServiceEntity) -> anyhow::Result<Out>;
}

#[async_trait::async_trait]
impl<Fut,In:DeserializeOwned,Out:Serialize,F:> JsonServiceExt<In,Out> for F
where F:Fn(Ctx,In,ServiceEntity)->Fut + Send+Sync+'static,
    Fut:Future<Output =anyhow::Result<Out>>+Send,
    In: DeserializeOwned+Send+Sync+'static,
    Out: Serialize+Send+Sync,
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
      In: DeserializeOwned+Send+Sync,
      Out: Serialize+Send+Sync,
{
    pub fn new(inner:T)->Self{
        Self{inner,_in:PhantomData::default(),_out:PhantomData::default()}
    }
}

#[async_trait::async_trait]
impl<T,In,Out> Service for JsonService<T,In,Out>
where T: JsonServiceExt<In,Out> + Sync + 'static,
    In: DeserializeOwned+Send+Sync,
    Out: Serialize+Send+Sync,
{
    async fn call(&self, ctx: Ctx, mut node: ServiceEntity) -> anyhow::Result<Output> {
        let input = self.inner.input(ctx.clone(),&mut node)?;
        let output = JsonServiceExt::call(&self.inner,ctx,input,node).await?;
        self.inner.output(output)
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