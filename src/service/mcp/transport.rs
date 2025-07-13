use std::time::Duration;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::service::mcp::McpError;

pub const JSONRPC_VERSION_V2: &'static str = "2.0";

#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(untagged)]
pub enum JsonRPCMessage{
    Req(JsonRPCMessageReq),
    Notice(JsonRPCMessageNotice),
    Resp(JsonRPCMessageResp),
    Error(JsonRPCMessageError),
}

impl JsonRPCMessage {
    pub fn to_json_str(&self) -> Result<String, McpError> {
        let s = serde_json::to_string(&self)?;Ok(s)
    }
    pub fn from_json_str(s: &str) -> Result<Self, McpError> {
        let s = serde_json::from_str(s)?;Ok(s)
    }
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct JsonRPCMessageReq{
    pub jsonrpc:String,
    pub id : RequestId,
    pub method : String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
}

impl RequestId {
    pub fn str<S: Into<String>>(s:S) -> Self {
        RequestId::String(s.into())
    }
    pub fn number(n: i64) -> Self {
        RequestId::Number(n)
    }
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct JsonRPCMessageNotice{
    pub jsonrpc:String,
    pub method : String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct JsonRPCMessageResp{
    pub jsonrpc:String,
    pub id : RequestId,
    pub result: Value,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct JsonRPCMessageError{
    pub jsonrpc:String,
    pub id : RequestId,
    pub error: JSONRPCErrorObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCErrorObject {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}


#[async_trait::async_trait]
pub trait Transport {
    // Send a message
    async fn send(&mut self, message: &JsonRPCMessage) -> Result<(), McpError>;

    /// Receive a message
    async fn receive(&mut self,timeout:Duration) -> Result<JsonRPCMessage, McpError>;

    /// Close the connection
    async fn close(&mut self) -> Result<(), McpError>;
}