use crate::core::env::EnvExt;
use crate::core::hook::FlowCallback;
use crate::core::service::{MapServiceLoader, Service, ServiceLoader};
use crate::core::{Ctx, CtxStatus, Error, Plan, RuntimePool, ServiceEntity, TokioRuntimePool};
use pin_project_lite::pin_project;
use std::any::{Any};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use wd_tools::PFErr;

pub struct EngineRT {
    pub service_loader: Box<dyn ServiceLoader + Sync + 'static>,
    pub service_middles: Vec<Arc<dyn Service + Sync + 'static>>,
    pub runtime_pool: Box<dyn RuntimePool + Sync + 'static>,
    pub flow_start_callback: Vec<Box<dyn FlowCallback + Sync + 'static>>,
    pub flow_end_callback: Vec<Box<dyn FlowCallback + Sync + 'static>>,
}

impl Default for EngineRT {
    fn default() -> Self {
        let service_loader = Box::new(MapServiceLoader::default());
        let service_middles = vec![];
        let runtime_pool = Box::new(TokioRuntimePool::default());
        let flow_start_callback = vec![];
        let flow_end_callback = vec![];
        Self {
            service_middles,
            service_loader,
            runtime_pool,
            flow_start_callback,
            flow_end_callback,
        }.append_service_middle(Engine::base_hook)
    }
}

impl EngineRT {
    pub fn set_service_loader<S: ServiceLoader + Sync + 'static>(mut self, loader: S) -> Self {
        self.service_loader = Box::new(loader);
        self
    }
    pub fn append_service_middle<S: Service + Sync + 'static>(mut self, middle: S) -> Self {
        self.service_middles.push(Arc::new(middle));
        self
    }
    pub fn set_runtime_pool<P: RuntimePool + Sync + 'static>(mut self, pool: P) -> Self {
        self.runtime_pool = Box::new(pool);
        self
    }
    pub fn append_start_callback<B: FlowCallback + Sync + 'static>(mut self, callback: B) -> Self {
        self.flow_start_callback.push(Box::new(callback));
        self
    }
    pub fn append_end_callback<B: FlowCallback + Sync + 'static>(mut self, callback: B) -> Self {
        self.flow_end_callback.push(Box::new(callback));
        self
    }
    pub fn build(self) -> Engine {
        Engine {
            entity: Arc::new(self),
        }
    }
}

pin_project! {
    struct WaitCallback{
    ctx:Ctx
}
}

impl From<Ctx> for WaitCallback {
    fn from(ctx: Ctx) -> Self {
        Self { ctx }
    }
}
impl Future for WaitCallback {
    type Output = anyhow::Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.ctx.deref_mut_metadata(|c| {
            return match c.status {
                CtxStatus::Init => {
                    c.waker = Some(cx.waker().clone());
                    c.status = CtxStatus::RUNNING;
                    Poll::Pending
                }
                CtxStatus::RUNNING => Poll::Ready(
                    anyhow::anyhow!("WaitCallback.status[RUNNING]: Abnormal wake up").err(),
                ),
                CtxStatus::SUCCESS => Poll::Ready(Ok(())),
                CtxStatus::Error => Poll::Ready(Ok(())),
                CtxStatus::Over => Poll::Ready(
                    anyhow::anyhow!("WaitCallback.status[Over]: Abnormal wake up").err(),
                ),
            };
        })
    }
}
pin_project! {
struct StartServiceFut{
        ctx:Ctx,
        start:bool,
        fut:Pin<Box<dyn Future<Output=()> + Send>>
}}
impl Future for StartServiceFut {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        if !this.start.deref() {
            *this.start.deref_mut() = this.ctx.deref_mut_metadata(|c| c.status == CtxStatus::Init);
            cx.waker().wake_by_ref();
            return Poll::Pending
        }
        this.fut.as_mut().poll(cx)
    }
}

#[derive(Clone)]
pub struct Engine {
    pub entity: Arc<EngineRT>,
}

impl Engine {
    pub fn ctx<P: Plan + Sync + 'static>(&self, p: P) -> Ctx {
        Ctx::new(self.clone(), p)
    }
    pub(crate) async fn ignore_err(ctx: Ctx, se: ServiceEntity) {
        if let Err(e) = ctx.clone().next(se).await {
            ctx.set_any_error(e).await;
        }
    }
    pub(crate) async fn call_service(ctx: Ctx, rt: Engine, se: ServiceEntity) {
        let fut = Self::ignore_err(ctx, se);
        rt.entity.runtime_pool.push(Box::pin(fut)).await;
    }
    pub(crate) async fn raw_run<In: Any + Send>(mut ctx: Ctx, input: In) -> anyhow::Result<()> {
        // ctx.get_env().feedback_ext(input).await?;
        ctx = ctx.insert_input(input);
        let start = ctx.unsafe_mut_plan(|c|c.start_node_name().to_string());
        let rt = ctx.rt.clone();
        let mut se = ctx.deref_mut_plan(|c|{
            let option = c.get(start.as_str());
                match option {
                    Some(o)=>Ok(o),
                    None => Err(Error::NodeEntityNotFound(start))
                }
        })?;
        //执行前置任务
        for i in rt.entity.flow_start_callback.iter() {
            i.call(ctx.clone()).await?;
        }
        //执行第一个service
        if let Some(s) = rt.entity.service_loader.load(se.service_name.as_str()).await {
            se = se.set_service(s);
        } else {
            return Err(Error::ServiceNotFound(se.service_name).into())
        }
        let next = Self::call_service(ctx.clone(), rt.clone(), se);
        let ssf = StartServiceFut {
            ctx: ctx.clone(),
            fut: Box::pin(next),
            start: false,
        };
        rt.entity.runtime_pool.push(Box::pin(ssf)).await;
        //等待结果返回
        WaitCallback::from(ctx.clone()).await?;
        //执行后置任务
        for i in rt.entity.flow_end_callback.iter().rev() {
            if let Err(err) = i.call(ctx.clone()).await {
                return Err(Error::EndCallbackError(err).into())
            }
        }
        Ok(())
    }
    pub async fn load_service(&self,name:&str)->Option<Arc<dyn Service + Sync + 'static>> {
        self.entity.service_loader.load(name).await
    }
    pub fn go<In: Any + Send>(ctx:Ctx,input: In){
        tokio::spawn(async move {
            if let Err(err) = Self::raw_run(ctx.clone(),input).await{
                ctx.set_any_error(err).await;
            }
        });
    }
    pub async fn run<In: Any + Send,Out:Any>(ctx: Ctx, input: In) -> anyhow::Result<Out>{
        Self::raw_run(ctx.clone(),input).await?;
        if ctx.get_status() == CtxStatus::SUCCESS {
            let end = ctx.unsafe_mut_plan(|c|c.end_node_name().to_string());
            let out = ctx.async_mut_metadata(|c|{
                let out = c.vars.remove(end.as_str());
                async move{out}
            }).await;
            return if let Some(s) = out{
                s.into()
            }else{
                anyhow::anyhow!("not found result").err()
            }
        }

        if let Some(e) = ctx.rem_error(){
            Err(anyhow::Error::from(e))
        }else{
            Err(anyhow::Error::from(Error::Unknown("not found error".into())))
        }
    }
}
