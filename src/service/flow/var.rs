use crate::core::{Ctx, Output, OutputObject, Service, ServiceEntity};
use serde_json::Value;
use std::any::{Any, TypeId};
use std::collections::HashMap;

#[async_trait::async_trait]
pub trait VarGenerator<T>: Send {
    async fn make(&self, ctx: Ctx, node: ServiceEntity) -> anyhow::Result<T>;
}
#[async_trait::async_trait]
impl<F, T> VarGenerator<T> for F
where
    F: Fn(Ctx, ServiceEntity) -> anyhow::Result<T> + Sync + Send,
{
    async fn make(&self, ctx: Ctx, node: ServiceEntity) -> anyhow::Result<T> {
        self(ctx, node)
    }
}

pub struct VarOut<T> {
    pub inner: T,
}
impl<T> VarOut<T> {
    pub fn new(inner: T) -> Self {
        VarOut { inner }
    }
}

impl<T: 'static + Send> OutputObject for VarOut<T> {
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
#[derive(Default)]
pub struct DefaultVarMap {
    pub map: HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>>,
}

pub struct Var<T> {
    pub generate: Box<dyn VarGenerator<T> + Send + Sync + 'static>,
}

impl<V> Var<V> {
    pub fn new<G: VarGenerator<V> + Send + Sync + 'static>(generate: G) -> Self {
        Var {
            generate: Box::new(generate),
        }
    }
    pub fn from_fn<F: Fn() -> V + Send + Sync + 'static>(generate: F) -> Self {
        Var::new(move |_c, _e| Ok(generate()))
    }
    pub fn from_default<T: Default>() -> Var<T> {
        Var::from_fn(|| T::default())
    }
    pub async fn raw_def_mut<T: 'static, Out, F: Fn(Option<&mut T>) -> Out>(
        ctx: &Ctx,
        node: &str,
        handle: F,
    ) -> Out {
        ctx.async_mut_metadata(|c| {
            let out = if let Some(out) = c.vars.get_mut(node) {
                if let Some(opt) = out.def_inner_mut::<VarOut<T>>() {
                    handle(Some(&mut opt.inner))
                } else {
                    handle(None)
                }
            } else {
                handle(None)
            };
            async { out }
        })
        .await
    }
    pub async fn def_mut<T: 'static, Out, F: Fn(Option<&mut T>) -> Out>(
        ctx: &Ctx,
        node: &str,
        handle: F,
    ) -> Out {
        Var::<V>::raw_def_mut::<DefaultVarMap, Out, _>(ctx, node, |o| {
            if let Some(s) = o {
                if let Some(s) = s.map.get_mut(&TypeId::of::<T>()) {
                    if let Some(s) = s.downcast_mut() {
                        return handle(Some(s));
                    }
                }
            }
            handle(None)
        })
        .await
    }
}

#[async_trait::async_trait]
impl<T: Send + Sync + 'static> Service for Var<T> {
    async fn call(&self, ctx: Ctx, node: ServiceEntity) -> anyhow::Result<Output> {
        let var = self.generate.make(ctx, node).await?;
        let var = Output::new(VarOut::new(var));
        Ok(var)
    }
}
impl<D: Default> Default for Var<D> {
    fn default() -> Self {
        Self::from_default()
    }
}
