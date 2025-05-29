use std::any::{Any, TypeId};
use serde_json::Value;
use crate::core::{Ctx, Output, OutputObject, Service, ServiceEntity};

#[async_trait::async_trait]
pub trait VarGenerator<T>: Send {
    async fn make(&self, ctx: Ctx, node: ServiceEntity) -> anyhow::Result<T>;
}

pub struct VarOut<T>{
    pub inner:T,
}
impl<T> VarOut<T> {
    pub fn new(inner:T) -> Self {
        VarOut{inner}
    }
}

impl<T: 'static + Send> OutputObject for VarOut<T>  {
    fn this_type_name(&self) -> &'static str {
        std::any::type_name::<VarOut<T>>()
    }

    fn this_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn get_val(&self, _key: &str) -> Option<Value> {
        None
    }

    fn any(self: Box<Self>) -> Box<dyn Any + Send + 'static> {
        self
    }
}

pub struct Var<T>{
    pub generate:Box<dyn VarGenerator<T>+Send+Sync+'static>,
}

impl<V> Var<V>{
    pub fn v_name()->String{
        format!("[Var::{}]",std::any::type_name::<V>())
    }
    pub async fn def<T:'static,Out,F:Fn(Option<&mut T>)->Out>(ctx:&Ctx,node:&str,handle:F)->Out{
        ctx.async_mut_metadata(|c|{
            let out = if let Some(out) = c.vars.get_mut(node){
                if let Some(opt) = out.def_inner_mut::<VarOut<T>>() {
                    handle(Some(&mut opt.inner))
                }else{
                    handle(None)
                }
            }else{
                handle(None)
            };
            async { out }
        }).await
    }
}


#[async_trait::async_trait]
impl<T:Send+Sync+'static> Service for Var<T> {
    async fn call(&self, ctx: Ctx, node: ServiceEntity) -> anyhow::Result<Output> {
        let var = self.generate.make(ctx, node).await?;
        let var = Output::new(VarOut::new(var));
        Ok(var)
    }
}