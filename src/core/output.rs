use serde_json::Value;
use std::any::{type_name, Any};
use std::fmt::{Debug, Formatter};
use wd_tools::PFErr;

pub trait OutputObject {
    fn type_name(&self)->&'static str{
        std::any::type_name::<Self>().into()
    }
    fn get(&self, key: &str) -> Option<Value>;
    fn set(&mut self, _key: &str, _val: Value) {
        panic!("default OutputObject not support set.")
    }
    fn string(&self) -> String {
        std::any::type_name::<Self>().into()
    }
    fn any(self:Box<Self>) ->Box<dyn Any>;
}


impl<T:Any> OutputObject for T {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>().into()
    }

    fn get(&self, _key: &str) -> Option<Value> {
        None
    }

    fn any(self:Box<Self>) -> Box<dyn Any> {
        self
    }
}


// pub type Output = Box<dyn OutputObject + Send + 'static>;
pub struct Output {
    pub inner: Box<dyn OutputObject + Send + 'static>,
}

impl Debug for Output {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Output type[{}]",self.inner.type_name())
    }
}

impl Output {
    pub fn new<T: OutputObject + Send + 'static>(t: T) -> Self {
        Output { inner: Box::new(t) }
    }
    pub fn into<T:'static>(self)->anyhow::Result<T>{
        let name = self.inner.type_name();
        match (self.inner).any().downcast::<T>(){
            Ok(o) => Ok(*o),
            Err(_e) => {
                anyhow::anyhow!("expect type[{}] found type[{:?}]",type_name::<T>(),name).err()
            }
        }
    }
}