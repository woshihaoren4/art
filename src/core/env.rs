use std::any::{Any, TypeId};
use std::collections::HashMap;
use wd_tools::sync::Am;

#[async_trait::async_trait]
pub trait Env: Send + Sync {
    async fn watch(&self, t: TypeId) -> anyhow::Result<Option<Box<dyn Any>>>;
    async fn feedback(&self, info: Box<dyn Any + Send>) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait EnvExt: Send + Sync {
    #[allow(unused)]
    async fn watch_ext<T: Any>(&self) -> anyhow::Result<Option<T>>;
    #[allow(unused)]
    async fn feedback_ext<T: Any + Send>(&self, info: T) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl<T: Env + ?Sized + 'static> EnvExt for T {
    async fn watch_ext<A: Any>(&self) -> anyhow::Result<Option<A>> {
        if let Some(s) = self.watch(TypeId::of::<T>()).await? {
            let a = s.downcast::<A>().unwrap();
            Ok(Some(*a))
        } else {
            Ok(None)
        }
    }

    async fn feedback_ext<A: Any + Send>(&self, info: A) -> anyhow::Result<()> {
        self.feedback(Box::new(info)).await
    }
}

pub struct CabinetEnv {
    pub cabinet: Am<HashMap<TypeId, Box<dyn Any>>>,
}
impl CabinetEnv {
    pub fn new() -> Self {
        Self {
            cabinet: Am::new(HashMap::new()),
        }
    }
}
#[async_trait::async_trait]
impl Env for CabinetEnv {
    async fn watch(&self, t: TypeId) -> anyhow::Result<Option<Box<dyn Any>>> {
        let mut lock = self.cabinet.lock().await;
        if let Some(x) = lock.remove(&t) {
            Ok(Some(x))
        } else {
            Ok(None)
        }
    }

    async fn feedback(&self, info: Box<dyn Any + Send>) -> anyhow::Result<()> {
        let key = (&info).type_id();
        let mut lock = self.cabinet.lock().await;
        lock.insert(key, Box::new(info));
        Ok(())
    }
}
