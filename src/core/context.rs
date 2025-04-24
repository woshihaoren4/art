use crate::core::env::{CabinetEnv, Env};
use crate::core::{Engine, Error, Output, Plan, ServiceEntity};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::ops::DerefMut;
use std::sync::Arc;
use std::task::Waker;
use wd_tools::sync::Am;

#[derive(Default, Copy, Clone)]
pub enum CtxStatus {
    #[default]
    Init,
    RUNNING,
    SUCCESS,
    Error,
    Over,
}
impl Display for CtxStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CtxStatus::Init => write!(f, "CtxStatus::New"),
            CtxStatus::RUNNING => write!(f, "CtxStatus::RUNNING"),
            CtxStatus::SUCCESS => write!(f, "CtxStatus::SUCCESS"),
            CtxStatus::Error => write!(f, "CtxStatus::Error"),
            CtxStatus::Over => write!(f, "CtxStatus::Over"),
        }
    }
}
impl PartialEq for CtxStatus {
    fn eq(&self, other: &Self) -> bool {
        match other {
            CtxStatus::Init => match self {
                CtxStatus::Init => true,
                _ => true,
            },
            CtxStatus::RUNNING => match self {
                CtxStatus::RUNNING => true,
                _ => false,
            },
            CtxStatus::SUCCESS => match self {
                CtxStatus::SUCCESS => true,
                _ => false,
            },
            CtxStatus::Error => match self {
                CtxStatus::Error => true,
                _ => false,
            },
            CtxStatus::Over => match self {
                CtxStatus::Over => true,
                _ => false,
            },
        }
    }
}

// pub struct StackNode{
//     pub parent : Vec<String>,
//     pub from: String,
//     pub to: String,
// }
// #[derive(Default)]
// pub struct Stack{
//
// }

pub struct Metadata {
    pub input: Option<Box<dyn Any>>,
    pub error: Option<Error>,
    pub status: CtxStatus,
    pub waker: Option<Waker>,
    // pub plan: Box<>,
    pub vars: HashMap<String, Output>,
    // pub env: Arc<dyn Env + 'static>,
    // pub stack :Stack
}
pub struct Ctx {
    pub ce: Arc<Am<Metadata>>,
    pub plan: Arc<Am<Box<dyn Plan + Sync + 'static>>>,
    pub env: Arc<dyn Env + 'static>,
    pub rt: Engine,
}
impl Clone for Ctx {
    fn clone(&self) -> Self {
        Self {
            rt: self.rt.clone(),
            plan: self.plan.clone(),
            ce: self.ce.clone(),
            env: self.env.clone(),
        }
    }
}

impl Ctx {
    pub async fn next(self, mut node: ServiceEntity) -> anyhow::Result<Output> {
        let rt = self.rt.clone();
        loop {
            if node.middle_index < rt.entity.service_middles.len() {
                let middle = &rt.entity.service_middles[node.middle_index];
                node.middle_index += 1;
                return middle.call(self, node).await;
            } else if node.middle_index == rt.entity.service_middles.len() {
                let service = node.service.clone();
                return service.call(self, node).await;
            } else {
                //Will not execute
                return Error::NextNodeNull.into();
            }
        }
    }
    pub fn new<P: Plan + Sync + 'static>(rt: Engine, plan: P) -> Self {
        let ctx = Metadata {
            error: None,
            input: None,
            status: Default::default(),
            waker: None,
            vars: Default::default(),
        };
        Self {
            rt,
            plan: Arc::new(Am::new(Box::new(plan))),
            env: Arc::new(CabinetEnv::new()),
            ce: Arc::new(Am::new(ctx)),
        }
    }
    pub async fn async_mut_metadata<
        Out,
        Fut: Future<Output = Out>,
        H: FnOnce(&mut Metadata) -> Fut,
    >(
        &self,
        function: H,
    ) -> Out {
        let mut lock = self.ce.lock().await;
        function(lock.deref_mut()).await
    }
    pub fn deref_mut_metadata<Out, H: FnOnce(&mut Metadata) -> Out>(&self, function: H) -> Out {
        let mut lock = self.ce.synchronize();
        function(lock.deref_mut())
    }
    pub fn unsafe_mut_metadata<Out, H: FnOnce(&mut Metadata) -> Out>(&self, function: H) -> Out {
        unsafe {
            let c = self.ce.raw_ptr_mut();
            function(&mut *c)
        }
    }
    pub async fn insert_var<N: Into<String>, T: Into<Output>>(&self, node: N, t: T) {
        self.async_mut_metadata(|c| {
            c.vars.insert(node.into(), t.into());
            async { () }
        })
        .await
    }
    pub async fn rm_var(&self, node: &str) -> Option<Box<dyn Any + Send + 'static>> {
        let out = self
            .async_mut_metadata(|c| {
                let out = c.vars.remove(node);
                async move { out }
            })
            .await;
        if let Some(s) = out {
            Some(s.into_any())
        } else {
            None
        }
    }
    pub async fn get_var_field(&self, node: &str, field: &str) -> Option<Value> {
        self.async_mut_metadata(|c| {
            let res = if let Some(val) = c.vars.get(node) {
                val.get_val(field)
            } else {
                None
            };
            async move { res }
        })
        .await
    }
    pub fn insert_input<I: Any>(&self, input: I) {
        self.deref_mut_metadata(|c| c.input = Some(Box::new(input)));
    }
    pub fn rem_input(&self) -> Option<Box<dyn Any>> {
        self.deref_mut_metadata(|c| c.input.take())
    }
    pub fn insert_error(&self, err: anyhow::Error) {
        let err = err
            .downcast::<Error>()
            .unwrap_or_else(|e| Error::AnyhowError(e));
        self.deref_mut_metadata(|c| c.error = Some(err));
    }
    pub fn rem_error(&self) -> Option<Error> {
        self.deref_mut_metadata(|c| c.error.take())
    }
    pub fn deref_mut_plan<Out, H: FnOnce(&mut Box<dyn Plan + Sync + 'static>) -> Out>(
        &self,
        function: H,
    ) -> Out {
        let mut lock = self.plan.synchronize();
        function(lock.deref_mut())
    }
    pub fn unsafe_mut_plan<Out, H: FnOnce(&mut Box<dyn Plan + Sync + 'static>) -> Out>(
        &self,
        function: H,
    ) -> Out {
        unsafe {
            let c = self.plan.raw_ptr_mut();
            function(&mut *c)
        }
    }
    pub fn clone_no_plan(&self) -> Self {
        let mut c = self.clone();
        c.plan = Arc::new(Am::new(Box::new(())));
        c
    }
    // pub(crate) fn set_waker(self, waker: Waker) -> Self {
    //     self.unsafe_mut_metadata(|c| c.waker = Some(waker));
    //     self
    // }
    pub fn set_env(mut self, env: Arc<dyn Env + 'static>) -> Self {
        self.env = env.clone();
        self
    }
    pub fn get_env(&self) -> Arc<dyn Env + 'static> {
        self.env.clone()
    }

    pub async fn set_any_error(&self, err: anyhow::Error) {
        let err = err
            .downcast::<Error>()
            .unwrap_or_else(|e| Error::AnyhowError(e));
        // if let Err(e) = self.get_env().feedback_ext(err).await {
        //     wd_log::log_field("error",e).error("ctx.next return error and to env failed");
        // }
        self.async_mut_metadata(|c| {
            c.error = Some(err);
            c.status = CtxStatus::Error;
            if let Some(w) = c.waker.take() {
                w.wake();
            }
            async { () }
        })
        .await;
    }
    pub async fn success(&self) {
        self.async_mut_metadata(|c| {
            c.status = CtxStatus::SUCCESS;
            if let Some(w) = c.waker.take() {
                w.wake();
            }
            async { () }
        })
        .await;
    }
    pub fn get_status(&self) -> CtxStatus {
        self.deref_mut_metadata(|c| c.status)
    }
    pub fn go<In: Any + Send>(self, input: In) {
        Engine::go(self, input)
    }
    pub async fn run<In: Any + Send, Out: Any>(self, input: In) -> anyhow::Result<Out> {
        Engine::run(self, input).await
    }
}
