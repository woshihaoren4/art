use serde_json::Value;
use std::any::{type_name, Any};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use wd_tools::{PFErr, PFOk};
use crate::core::Ctx;

pub trait OutputObject {
    fn type_name(&self)->&'static str{
        std::any::type_name::<Self>().into()
    }
    fn get_val(&self, key: &str) -> Option<Value>;
    fn set_value(&mut self, _key: &str, _val: Value) {
        panic!("default OutputObject not support set.")
    }
    fn string(&self) -> String {
        std::any::type_name::<Self>().into()
    }
    fn any(self:Box<Self>) ->Box<dyn Any+Send+'static>;
}

// impl<T: Any> OutputObject for T {
//     fn type_name(&self) -> &'static str {
//         std::any::type_name::<Self>().into()
//     }
//
//     fn get_val(&self, _key: &str) -> Option<Value> {
//         None
//     }
//
//     fn any(self:Box<Self>) -> Box<dyn Any> {
//         self
//     }
// }
impl OutputObject for Value {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Value>().into()
    }
    fn get_val(&self, key: &str) -> Option<Value>{
        let ks = key.splitn(2,".").collect::<Vec<_>>();
        if let Value::Object(obj) = self {
            if let Some(val) = obj.get(ks[0]) {
                if ks.len() == 1 {
                    return Some(val.clone())
                }else{
                    return Self::get_val(val,ks[1])
                }
            }
        }
        None
    }
    fn set_value(&mut self, key: &str, val: Value) {
        let ss = key.splitn(2, ".").collect::<Vec<_>>();
        if let Value::Object(_) = self{

        }else{
            *self = Value::Object(serde_json::Map::new());
        }
        if let Value::Object(obj) = self{
            if ss.len() == 1 {
                obj.insert(ss[0].to_string(),val);
            }else{
                if obj.get_mut(ss[0]).is_none() {
                    obj.insert(ss[0].to_string(),Value::Object(serde_json::Map::new()));
                }
                if let Some(s) = obj.get_mut(ss[0]) {
                    Self::set_value(s,ss[1],val)
                }else{
                    panic!("OutputObject for Value not a must have field");
                }
            }
        }
    }
    fn string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e|e.to_string())
    }
    fn any(self:Box<Self>) ->Box<dyn Any+Send+'static>{
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
impl<T:OutputObject + Send + 'static> From<T> for Output {
    fn from(value: T) -> Self {
        Output::new(value)
    }
}
impl Default for Output {
    fn default() -> Self {
        Self::new(Value::Null)
    }
}
impl Output {
    pub fn value<V:Into<Value>>(v:V)->Self{
        let value = v.into();
        Self::new(value)
    }
    pub fn json(t: impl Serialize)->anyhow::Result<Self>{
        let val = serde_json::to_value(t)?;
        Self::new(val).ok()
    }
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
    pub fn get_val(&self, key: &str) -> Option<Value>{
        self.inner.get_val(key)
    }
    pub fn set_value(&mut self, key: &str, val: Value) {
        self.inner.set_value(key,val)
    }
    pub fn into_any(self)->Box<dyn Any+Send+'static>{
        self.inner.any()
    }
}


#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tran{
    Value(Value),
    Quote(String),
}

impl Tran {
    pub fn value<V:Into<Value>>(val:V)->Tran{
        Tran::Value(val.into())
    }
    pub fn quote<S:Into<String>>(quote:S)->Self{
        Tran::Quote(quote.into())
    }
}

#[derive(Default,Debug,Clone,Serialize,Deserialize)]
#[serde(default)]
pub struct JsonInput{
    none_quote_skip:bool,
    transform_rule: HashMap<String,Tran>,
}

impl JsonInput {
    pub fn skip_null_quote(mut self)->Self{
        self.none_quote_skip = true;self
    }
    pub fn add_transform_rule<S:Into<String>,I:Into<Tran>>(mut self, position:S, transform:I) ->Self{
        let pos = position.into();
        let tran = transform.into();
        self.transform_rule.insert (pos, tran);
        self
    }
    pub fn add_transform_rules<S:Into<String>,T:Into<Tran>,I:IntoIterator<Item=(S,T)>>(mut self,iter : I)->Self{
        for (p,t) in iter {
            self = self.add_transform_rule(p,t);
        }
        self
    }
    pub fn add_transform_value<S:Into<String>,V:Into<Value>>(self, position:S, transform:V) ->Self{
        self.add_transform_rule(position,Tran::value(transform))
    }
    pub fn add_transform_quote<S:Into<String>,V:Into<String>>(self, position:S, transform:V) ->Self{
        self.add_transform_rule(position,Tran::quote(transform))
    }
    pub fn insert_val_to_json_val(t:&mut Value,pos:&str,val:Value)->anyhow::Result<()>{
        let ss = pos.splitn(2, ".").collect::<Vec<_>>();
        match t {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_)  => {
                anyhow::anyhow!("JsonInput.to not found object field[{}]",ss[0]).err()
            }
            Value::Object(map) => {
                if ss.len() == 1 {
                    map.insert(ss[0].to_string(),val);
                }else{
                    return Self::insert_val_to_json_val(t,ss[1],val);
                }
                Ok(())
            }
        }
    }
    pub fn remove_val_from_json_val(t:&mut Value,pos:&str)->anyhow::Result<Value>{
        if pos == "*" {
            let val = std::mem::replace(t,Value::Null);
            return Ok(val)
        }
        let ss = pos.splitn(2, ".").collect::<Vec<_>>();
        match t {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_)  => {
                return anyhow::anyhow!("JsonInput.remove_val_from_json_val field[{}]",ss[0]).err()
            }
            Value::Object(map) => {
                if ss.len() == 1 {
                    if let Some(val) = map.remove(ss[0]){
                        return Ok(val)
                    }
                }else{
                    if let Some(val) = map.get_mut(ss[0]){
                        return Self::remove_val_from_json_val(val,ss[1])
                    }
                }
                return anyhow::anyhow!("JsonInput.remove_val_from_json_val not found field[{}]",ss[0]).err()
            }
        }
    }
    pub fn transform_value(self,tar:&mut Value,mut scr:Value)->anyhow::Result<()>{
        for (k,v) in self.transform_rule {
            let value = match v {
                Tran::Value(v) => v,
                Tran::Quote(q) => {
                    if let Ok(val) = Self::remove_val_from_json_val(&mut scr, q.as_str()){
                        val
                    }else{
                        if self.none_quote_skip {
                            continue
                        }else{
                            return anyhow::anyhow!("JsonInput.transform_value not found node.field[{}]",q).err()
                        }
                    }
                }
            };
            Self::insert_val_to_json_val(tar,k.as_str(),value)?;
        }
        Ok(())
    }
    pub async fn transform<T:Serialize+DeserializeOwned>(self, ctx:Ctx, val:T) ->anyhow::Result<T>{
        let mut val = serde_json::to_value(val)?;
        for (k,v) in self.transform_rule {
            let value = match v {
                Tran::Value(v) => v,
                Tran::Quote(q) => {
                    let ss = q.splitn(2,".").collect::<Vec<_>>();
                    let node = ss[0];
                    let key = if ss.len() > 1 {
                        ss[1]
                    }else{
                        ""
                    };
                    if let Some(val) =  ctx.get_var_field(node, key).await{
                        val
                    }else{
                        if self.none_quote_skip {
                            continue
                        }else{
                           return anyhow::anyhow!("JsonInput.to not found node.field[{}] from metadata",q).err()
                        }
                    }
                }
            };
            println!("--->{}",val);
            Self::insert_val_to_json_val(&mut val,k.as_str(),value)?;
        }
        let t = serde_json::from_value::<T>(val)?;Ok(t)
    }
    pub async fn default_transform<T:Serialize+DeserializeOwned+Default>(self, ctx:Ctx) ->anyhow::Result<T>{
        let val = T::default();
        self.transform(ctx,val).await
    }
}

impl TryFrom<&str> for JsonInput{
    type Error = serde_json::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        serde_json::from_str::<JsonInput>(value)
    }
}

#[cfg(test)]
mod test{
    use serde::{Deserialize, Serialize};
    use crate::core::{Ctx, EngineRT, JsonInput};

    #[derive(Default,Debug,Serialize,Deserialize)]
    struct TestJson{
        name: String,
        code : isize,
        list:Vec<isize>
    }

    #[tokio::test]
    async fn test_input(){
        let json = serde_json::json!({
            "code":1,
            "message":"success",
            "data":{
                "list":[1,2,3]
            }
        });
        let ctx = Ctx::new(EngineRT::default().build(),());
        ctx.insert_var("test_node",json).await;

        let ji = JsonInput::default().skip_null_quote()
            .add_transform_value("name", "helloworld")
            .add_transform_quote("message", "test_node.message")
            .add_transform_quote("code", "test_node.code_v2")
            .add_transform_quote("list", "test_node.data.list");

        let t = ji.default_transform::<TestJson>(ctx).await.unwrap();
        assert_eq!(t.code,0);
        assert_eq!(t.name,"helloworld");
        assert_eq!(t.list[0],1);
        assert_eq!(t.list[2],3);
    }

    #[test]
    fn test_json_input_from(){
        // let ji = JsonInput::default().skip_null_quote()
        //     .add_transform_value("name", "helloworld")
        //     .add_transform_quote("message", "test_node.message")
        //     .add_transform_quote("code", "test_node.code_v2")
        //     .add_transform_quote("list", "test_node.data.list");
        // println!("{}",serde_json::to_string(&ji).unwrap());
        let ji = JsonInput::try_from(r#"
        {
            "none_quote_skip":true,
            "transform_rule":{"message":{"quote":"test_node.message"},"name":{"value":"helloworld"},"code":{"quote":"test_node.code_v2"},"list":{"quote":"test_node.data.list"}}
        }
        "#).unwrap();
        println!("{:?}",ji)
    }
}