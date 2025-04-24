use crate::core::{EmptyServiceImpl, Service};
use std::any::Any;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

pub struct ServiceEntity {
    pub(crate) middle_index: usize,
    pub(crate) service: Arc<dyn Service + Sync + 'static>,

    pub service_name: String,
    pub node_name: String,
    pub config: Box<dyn Any + Send + Sync + 'static>,
}
impl Display for ServiceEntity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "service_name:{},node_name:{}",
            self.service_name, self.node_name
        )
    }
}
impl Debug for ServiceEntity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
impl Default for ServiceEntity {
    fn default() -> Self {
        Self {
            middle_index: 0,
            service: Arc::new(EmptyServiceImpl),
            service_name: "".to_string(),
            node_name: "".to_string(),
            config: Box::new(()),
        }
    }
}
impl<N: Into<String>, C: Any + Send + Sync + 'static> From<(N, C)> for ServiceEntity {
    fn from((n, c): (N, C)) -> Self {
        Self::new(c).set_service_name(n)
    }
}
impl From<&str> for ServiceEntity {
    fn from(value: &str) -> Self {
        Self::from((value.to_string(), value.to_string()))
    }
}

impl ServiceEntity {
    // pub(crate) fn set_service<S:Service + Sync + 'static>(mut self,service:S)->Self{
    //     self.service = Arc::new(service);self
    // }
    pub(crate) fn set_service(mut self, service: Arc<dyn Service + Sync + 'static>) -> Self {
        self.service = service;
        self
    }
    pub fn new<A: Any + Send + Sync + 'static>(cfg: A) -> Self {
        Self::default().set_config(cfg)
    }
    pub fn set_node_name<S: Into<String>>(mut self, name: S) -> Self {
        self.node_name = name.into();
        self
    }
    pub fn set_service_name<S: Into<String>>(mut self, name: S) -> Self {
        self.service_name = name.into();
        self
    }
    pub fn set_config<A: Any + Send + Sync + 'static>(mut self, config: A) -> Self {
        self.config = Box::new(config);
        self
    }
    pub fn deref_mut_transform_config<F, T: Any, Out>(&mut self, transform_func: F) -> Out
    where
        F: FnOnce(Option<&T>) -> Out,
    {
        let t = if self.config.is::<T>() {
            self.config.downcast_mut()
        } else {
            None
        };
        transform_func(t.as_deref())
    }
    pub fn transform_config<F, T: Any, Out>(&mut self, transform_func: F) -> Out
    where
        F: FnOnce(Option<T>) -> Out,
    {
        let t = if self.config.is::<T>() {
            let val = std::mem::replace(&mut self.config, Box::new(()));
            val.downcast::<T>().map(|x| Some(*x)).unwrap_or(None)
        } else {
            None
        };
        transform_func(t)
    }
}
