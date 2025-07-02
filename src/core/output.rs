use crate::core::Ctx;
use crate::utils::string;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::str::FromStr;
use wd_tools::{PFErr, PFOk};

pub trait OutputObject:Any
where
    Self: 'static,
{
    fn this_type_name(&self) -> &'static str;
    fn this_type_id(&self) -> TypeId;
    fn get_val(&self, _key: &str) -> Option<Value>{
        None
    }
    fn as_val(&self)->Value{
        Value::Null
    }
    fn set_value(&mut self, _key: &str, _val: Value) {
        panic!("default OutputObject not support set.")
    }
    fn string(&self) -> String {
        std::any::type_name::<Self>().into()
    }
    fn any(self: Box<Self>) -> Box<dyn Any + Send + 'static>;
}

#[macro_export]
macro_rules! default_output_object {
    ($($stu:tt),*) => {
        $(
        impl crate::core::OutputObject for $stu {
            fn this_type_name(&self) -> &'static str {
                std::any::type_name::<$stu>().into()
            }
            fn this_type_id(&self) -> TypeId {
                std::any::TypeId::of::<$stu>()
            }
            fn any(self: Box<Self>) -> Box<dyn Any + Send + 'static> {
                self
            }
        }
        )*

    };
}
default_output_object!(i8,i16,i32,i64,isize,u8,u16,u32,u64,usize,f32,f64,bool,String);
// pub trait OutputObjectAny{
//     pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
//         if self.is::<T>() {
//             // SAFETY: just checked whether we are pointing to the correct type, and we can rely on
//             // that check for memory safety because we have implemented Any for all types; no other
//             // impls can exist as they would conflict with our impl.
//             unsafe { Some(self.downcast_ref_unchecked()) }
//         } else {
//             None
//         }
//     }
// }

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
    fn this_type_name(&self) -> &'static str {
        std::any::type_name::<Value>().into()
    }

    fn this_type_id(&self) -> TypeId {
        std::any::TypeId::of::<Value>()
    }

    fn get_val(&self, key: &str) -> Option<Value> {
        if key=="*" {
            return Some(self.clone());
        }
        let ks = key.splitn(2, ".").collect::<Vec<_>>();
        if let Value::Object(obj) = self {
            if let Some(val) = obj.get(ks[0]) {
                if ks.len() == 1 {
                    return Some(val.clone());
                } else {
                    return Self::get_val(val, ks[1]);
                }
            }
        }
        None
    }
    fn set_value(&mut self, key: &str, val: Value) {
        let ss = key.splitn(2, ".").collect::<Vec<_>>();
        if let Value::Object(_) = self {
        } else {
            *self = Value::Object(serde_json::Map::new());
        }
        if let Value::Object(obj) = self {
            if ss.len() == 1 {
                obj.insert(ss[0].to_string(), val);
            } else {
                if obj.get_mut(ss[0]).is_none() {
                    obj.insert(ss[0].to_string(), Value::Object(serde_json::Map::new()));
                }
                if let Some(s) = obj.get_mut(ss[0]) {
                    Self::set_value(s, ss[1], val)
                } else {
                    panic!("OutputObject for Value not a must have field");
                }
            }
        }
    }
    fn as_val(&self) -> Value {
        self.clone()
    }
    fn string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| e.to_string())
    }
    fn any(self: Box<Self>) -> Box<dyn Any + Send + 'static> {
        self
    }
}

// pub type Output = Box<dyn OutputObject + Send + 'static>;
pub struct Output {
    pub inner: Box<dyn OutputObject + Send + 'static>,
}

impl Debug for Output {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Output type[{}]", self.inner.this_type_name())
    }
}
impl<T: OutputObject + Send + 'static> From<T> for Output {
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
    pub fn value<V: Into<Value>>(v: V) -> Self {
        let value = v.into();
        Self::new(value)
    }
    pub fn json(t: impl Serialize) -> anyhow::Result<Self> {
        let val = serde_json::to_value(t)?;
        Self::new(val).ok()
    }
    pub fn new<T: OutputObject + Send + 'static>(t: T) -> Self {
        Output { inner: Box::new(t) }
    }
    pub fn assert<T: 'static>(&self) -> bool {
        self.inner.this_type_id() == TypeId::of::<T>()
    }
    pub fn inner_downcast_def<T:'static>(&self) -> Option<&T> {
        if !self.assert::<T>() {
            return None;
        }
        unsafe {
            let t = &*(&self.inner as *const Box<dyn OutputObject + Send + 'static> as *const Box<T>);
            Some(&*t)
        }
    }
    pub fn inner_downcast_mut<T:'static>(&mut self) -> Option<&mut T> {
        if !self.assert::<T>() {
            return None;
        }
        unsafe {
            let t = &mut *(&mut self.inner as *mut Box<dyn OutputObject + Send + 'static> as *mut Box<T>);
            Some(&mut *t)
        }
    }
    pub fn into<T: 'static>(self) -> anyhow::Result<T> {
        let name = self.inner.this_type_name();
        match (self.inner).any().downcast::<T>() {
            Ok(o) => Ok(*o),
            Err(_e) => {
                anyhow::anyhow!("expect type[{}] found type[{:?}]", type_name::<T>(), name).err()
            }
        }
    }
    pub fn def_inner_mut<T: 'static>(&mut self) -> Option<&mut T> {
        if !self.assert::<T>() {
            return None;
        }
        unsafe {
            let a = &mut *self.inner;
            let b = &mut *(a as *mut dyn OutputObject as *mut T as *mut T);
            Some(b)
        }
    }

    pub fn get_val(&self, key: &str) -> Option<Value> {
        self.inner.get_val(key)
    }
    pub fn as_val(&self) -> Value {
        self.inner.as_val()
    }
    pub fn set_value(&mut self, key: &str, val: Value) {
        self.inner.set_value(key, val)
    }
    pub fn into_any(self) -> Box<dyn Any + Send + 'static> {
        self.inner.any()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tran {
    Value(Value),
    Quote(String),
    Format(Vec<String>),
}

impl Tran {
    pub fn value<V: Into<Value>>(val: V) -> Tran {
        Tran::Value(val.into())
    }
    pub fn quote<S: Into<String>>(quote: S) -> Self {
        Tran::Quote(quote.into())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JsonInput {
    none_quote_skip: bool,
    transform_rule: HashMap<String, Tran>,
    default_json: Value, // 通过${{xxx.xxx}}的形式插入变量
}

impl JsonInput {
    pub fn skip_null_quote(mut self) -> Self {
        self.none_quote_skip = true;
        self
    }
    pub fn set_default_json(mut self, default_json: Value) -> Self {
        self.default_json = default_json;
        self
    }
    pub fn add_transform_rule<S: Into<String>, I: Into<Tran>>(
        mut self,
        position: S,
        transform: I,
    ) -> Self {
        let pos = position.into();
        let tran = transform.into();
        self.transform_rule.insert(pos, tran);
        self
    }
    pub fn add_transform_rules<S: Into<String>, T: Into<Tran>, I: IntoIterator<Item = (S, T)>>(
        mut self,
        iter: I,
    ) -> Self {
        for (p, t) in iter {
            self = self.add_transform_rule(p, t);
        }
        self
    }
    pub fn add_transform_value<S: Into<String>, V: Into<Value>>(
        self,
        position: S,
        transform: V,
    ) -> Self {
        self.add_transform_rule(position, Tran::value(transform))
    }
    pub fn add_transform_quote<S: Into<String>, V: Into<String>>(
        self,
        position: S,
        transform: V,
    ) -> Self {
        self.add_transform_rule(position, Tran::quote(transform))
    }
    pub fn insert_val_to_json_val(t: &mut Value, pos: &str, val: Value) -> anyhow::Result<()> {
        let ss = pos.splitn(2, ".").collect::<Vec<_>>();
        match t {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                println!("插入位置 --->{}", pos);
                anyhow::anyhow!(
                    "JsonInput.insert_val_to_json_val not found object field[{}]",
                    ss[0]
                )
                .err()
            }
            Value::Array(list) => {
                let index = if let Ok(o) = usize::from_str(ss[0]) {
                    o
                } else {
                    return anyhow::anyhow!(
                        "JsonInput.insert_val_to_json_val the array not support pos[{}]",
                        ss[0]
                    )
                    .err();
                };
                if ss.len() == 1 {
                    if index < list.len() {
                        list[index] = val;
                    } else {
                        list.push(val)
                    }
                    return Ok(());
                }
                return if index < list.len() {
                    Self::insert_val_to_json_val(&mut list[index], ss[1], val)
                } else {
                    anyhow::anyhow!(
                        "JsonInput.insert_val_to_json_val index[{}] >= array.len",
                        ss[0]
                    )
                    .err()
                };
            }
            Value::Object(map) => {
                if ss.len() == 1 {
                    map.insert(ss[0].to_string(), val);
                } else {
                    if let Some(m) = map.get_mut(ss[0]) {
                        Self::insert_val_to_json_val(m, ss[1], val)?;
                    } else {
                        let mut new_val = Value::Object(Map::new());
                        Self::insert_val_to_json_val(&mut new_val, ss[1], val)?;
                        map.insert(ss[0].to_string(), new_val);
                    }
                }
                Ok(())
            }
        }
    }
    pub fn format_val_to_json_str(
        t: &mut Value,
        pos: &str,
        val_name: String,
        val: Value,
    ) -> anyhow::Result<()> {
        let ss = pos.splitn(2, ".").collect::<Vec<_>>();
        match t {
            Value::String(s) => {
                *s = s.replace(val_name.as_str(), val.to_string().as_str());
                Ok(())
            }
            _ => anyhow::anyhow!("JsonInput.to format only support string, pos[{}]", ss[0]).err(),
        }
    }
    pub fn remove_val_from_json_val(t: &mut Value, pos: &str) -> anyhow::Result<Value> {
        if pos == "*" {
            let val = std::mem::replace(t, Value::Null);
            return Ok(val);
        }
        let ss = pos.splitn(2, ".").collect::<Vec<_>>();
        match t {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                return anyhow::anyhow!("JsonInput.remove_val_from_json_val field[{}]", ss[0]).err()
            }
            Value::Array(list) => {
                let index = if let Ok(o) = usize::from_str(ss[0]) {
                    o
                } else {
                    return anyhow::anyhow!(
                        "JsonInput.remove_val_from_json_val the array not support pos[{}]",
                        ss[0]
                    )
                    .err();
                };
                if index < list.len() {
                    if ss.len() == 1 {
                        let val = list.remove(index);
                        return Ok(val);
                    }
                    return Self::remove_val_from_json_val(&mut list[index], ss[1]);
                }
                return anyhow::anyhow!(
                    "JsonInput.remove_val_from_json_val pos[{}] >= array.len",
                    ss[0]
                )
                .err();
            }
            Value::Object(map) => {
                if ss.len() == 1 {
                    if let Some(val) = map.remove(ss[0]) {
                        return Ok(val);
                    }
                } else {
                    if let Some(val) = map.get_mut(ss[0]) {
                        return Self::remove_val_from_json_val(val, ss[1]);
                    }
                }
                return anyhow::anyhow!(
                    "JsonInput.remove_val_from_json_val not found field[{}]",
                    ss[0]
                )
                .err();
            }
        }
    }

    fn default_json_make_rule(&mut self, val: &mut Value, path: String) -> bool {
        match val {
            Value::String(s) => {
                if self.transform_rule.contains_key(path.as_str()) {
                    return false;
                }
                let mut list = string::extract_template_content(s);
                if list.len() == 1 && list[0].len() == (s.len() - 5) {
                    self.transform_rule
                        .insert(path, Tran::Quote(list.remove(0)));
                    return true;
                } else if list.len() > 0 {
                    self.transform_rule.insert(path, Tran::Format(list));
                }
            }
            Value::Array(list) => {
                for (i, v) in list.iter_mut().enumerate() {
                    let p = if path.is_empty() {
                        path.clone()
                    } else {
                        format!("{}.{}", path, i)
                    };
                    self.default_json_make_rule(v, p);
                }
            }
            Value::Object(obj) => {
                let mut remove_list = vec![];
                for (k, v) in obj.iter_mut() {
                    let p = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", path, k)
                    };
                    if self.default_json_make_rule(v, p) {
                        remove_list.push(k.clone())
                    }
                }
                for i in remove_list {
                    obj.remove(i.as_str());
                }
            }
            _ => {}
        }
        return false;
    }
    fn cover_default(default: Value, val: &mut Value) {
        match default {
            Value::Null => {
                return;
            }
            Value::Object(obj) => {
                for (k, v) in obj {
                    if let Value::Object(map) = val {
                        if let Some(val) = map.get_mut(k.as_str()) {
                            Self::cover_default(v, val);
                        } else {
                            map.insert(k, v);
                        }
                    } else {
                        let mut map = Map::new();
                        map.insert(k, v);
                        *val = Value::Object(map);
                    }
                }
            }
            _ => {
                *val = default;
            }
        }
    }
    async fn get_var_from_ctx(
        pos: &str,
        ctx: &Ctx,
        data_source: &mut Option<Value>,
    ) -> anyhow::Result<Value> {
        if let Some(val) = data_source {
            return Self::remove_val_from_json_val(val, pos);
        }
        let ss = pos.splitn(2, ".").collect::<Vec<_>>();
        let node = ss[0];
        let res = if ss.len() > 1 {
            if let Some(val) = ctx.get_var_field(node, ss[1]).await {
                val
            }else{
                return anyhow::anyhow!("JsonInput.to not found node.field[{}] from metadata", pos)
                    .err();
            }
            
        } else { 
            ctx.get_var(node).await
        };
        Ok(res)
    }
    pub async fn transform(
        mut self,
        ctx: Ctx,
        val: &mut Value,
        mut data_source: Option<Value>,
    ) -> anyhow::Result<()> {
        let mut default_json = self.default_json.take();
        self.default_json_make_rule(&mut default_json, "".into());
        Self::cover_default(default_json, val);

        for (k, v) in self.transform_rule {
            match v {
                Tran::Value(v) => {
                    Self::insert_val_to_json_val(val, k.as_str(), v)?;
                }
                Tran::Quote(q) => {
                    match Self::get_var_from_ctx(&q, &ctx, &mut data_source).await {
                        Ok(v) => {
                            Self::insert_val_to_json_val(val, k.as_str(), v)?;
                        }
                        Err(e) => {
                            if !self.none_quote_skip {
                                return Err(e);
                            }
                        }
                    };
                }
                Tran::Format(list) => {
                    for i in list {
                        match Self::get_var_from_ctx(i.as_str(), &ctx, &mut data_source).await {
                            Ok(v) => {
                                Self::format_val_to_json_str(
                                    val,
                                    k.as_str(),
                                    format!("${{{{{}}}}}", i),
                                    v,
                                )?;
                            }
                            Err(e) => {
                                if !self.none_quote_skip {
                                    return Err(e);
                                }
                            }
                        };
                    }
                }
            };
        }
        Ok(())
    }
    pub async fn default_transform<T: Serialize + DeserializeOwned + Default>(
        self,
        ctx: Ctx,
    ) -> anyhow::Result<T> {
        let val = T::default();
        let mut val = serde_json::to_value(val)?;
        self.transform(ctx, &mut val, None).await?;
        let val = match serde_json::from_value(val) {
            Ok(val) => val,
            Err(e) => {
                return anyhow::anyhow!("JsonInput.default_transform make t from value error:{e}")
                    .err()
            }
        };
        Ok(val)
    }
}

impl TryFrom<&str> for JsonInput {
    type Error = serde_json::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        serde_json::from_str::<JsonInput>(value)
    }
}

#[cfg(test)]
mod test {
    use crate::core::{Ctx, EngineRT, JsonInput, Output, OutputObject};
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use std::any::{Any, TypeId};
    use std::collections::HashMap;

    #[derive(Default, Debug, Serialize, Deserialize)]
    struct TestJson {
        name: String,
        code: isize,
        list: Vec<isize>,
        map: HashMap<String, isize>,
    }

    #[tokio::test]
    async fn test_input() {
        let json = serde_json::json!({
            "code":1,
            "message":"success",
            "data":{
                "list":[1,2,3]
            }
        });
        let ctx = Ctx::new(EngineRT::default().build(), ());
        ctx.insert_var("test_node", json).await;

        let ji = JsonInput::default()
            .skip_null_quote()
            .set_default_json(json!({
                "code":2,
                "map":{
                    "code2":"${{test_node.code}}"
                }
            }))
            .add_transform_value("name", "helloworld")
            .add_transform_quote("message", "test_node.message")
            .add_transform_quote("code", "test_node.code_v2")
            .add_transform_quote("map.code1", "test_node.code")
            .add_transform_quote("list", "test_node.data.list");

        let t = ji.default_transform::<TestJson>(ctx).await.unwrap();
        assert_eq!(t.code, 2);
        assert_eq!(t.name, "helloworld");
        assert_eq!(t.list[0], 1);
        assert_eq!(t.list[2], 3);
        println!("--->{:?}", t)
    }

    #[test]
    fn test_json_input_from() {
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
        println!("{:?}", ji)
    }
    #[test]
    fn test_output_into() {
        struct Req {
            name: String,
        }
        impl OutputObject for Req {
            fn this_type_name(&self) -> &'static str {
                std::any::type_name::<Req>()
            }

            fn this_type_id(&self) -> TypeId {
                TypeId::of::<Req>()
            }

            fn get_val(&self, _key: &str) -> Option<Value> {
                None
            }

            fn as_val(&self) -> Value {
                Value::Null
            }

            fn any(self: Box<Self>) -> Box<dyn Any + Send + 'static> {
                self
            }
        }

        let req = Req {
            name: "hello world".to_string(),
        };
        let mut out = Output::new(req);

        assert_eq!(out.assert::<Req>(), true);

        if let Some(s) = out.def_inner_mut::<Req>() {
            s.name = "hello thank you".into();
        }

        let res = out.into::<Req>();

        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap().name, "hello thank you");
    }
}
