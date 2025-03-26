use crate::core::{Ctx, Output, Service, ServiceEntity};
use std::future::Future;
use std::marker::PhantomData;

#[async_trait::async_trait]
impl<Fut, T> Service for T
where
    T: Fn(Ctx, ServiceEntity) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<Output>> + Send,
{
    async fn call(&self, ctx: Ctx, node: ServiceEntity) -> anyhow::Result<Output> {
        self(ctx, node).await
    }
}

pub struct FnServiceLayer<Fut,T>{
    inner:Box<T>,
    _f:PhantomData<Fut>,
}
#[async_trait::async_trait]
impl<Fut,T> Service for FnServiceLayer<Fut,T>
where
    T: Fn(Ctx, ServiceEntity) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<Output>> + Send + Sync,
{
    async fn call(&self, ctx: Ctx, node: ServiceEntity) -> anyhow::Result<Output> {
        (self.inner)(ctx,node).await
    }
}

impl<Fut,T> FnServiceLayer<Fut,T>{
    pub fn new(t:T)->Self{
        Self{inner:Box::new(t),_f:PhantomData::default()}
    }
}

#[cfg(test)]
mod test {
    use crate::core::{Output, Service};
    use crate::service::custom::function::FnServiceLayer;

    async fn fn_impl_service<S: Service + 'static>(ser: S) -> Box<dyn Service> {
        Box::new(ser)
    }
    #[tokio::test]
    async fn test_fn_impl_service() {
        let _ser = fn_impl_service(|_c, _s| async move { Ok(Output::default()) });
        let ser = FnServiceLayer::new(|_c,_n|async move{ Ok(Output::default())});
        let _ser = fn_impl_service(ser);
    }
}
