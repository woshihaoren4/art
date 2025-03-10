use serde_json::Value;
use std::any::{type_name, Any};
use wd_tools::PFErr;

pub struct Pipeline {}

pub trait OutputObject {
    fn get(&self, key: &str) -> Option<Value>;
    fn set(&mut self, _key: &str, _val: Value) {
        panic!("default OutputObject not support set.")
    }
    fn string(&self) -> String {
        std::any::type_name::<Self>().into()
    }
    fn any(self)->Box<dyn Any>;
}

impl<T:Any> OutputObject for T {
    fn get(&self, _key: &str) -> Option<Value> {
        None
    }

    fn any(self) -> Box<dyn Any> {
        Box::new(self)
    }
}

// pub type Output = Box<dyn OutputObject + Send + 'static>;
pub struct Output {
    inner: Box<dyn OutputObject + Send + 'static>,
}

impl Output {
    pub fn new<T: OutputObject + Send + 'static>(t: T) -> Self {
        Output { inner: Box::new(t) }
    }
    pub fn into<T:'static>(self)->anyhow::Result<T>{
        match (self.inner).any().downcast::<T>(){
            Ok(o) => Ok(*o),
            Err(e) => {
                anyhow::anyhow!("expect type[{}] found type[{:?}]",type_name::<T>(),e.type_id()).err()
            }
        }
    }
}