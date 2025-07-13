use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::time::Timeout;
use wd_tools::PFOk;
use log::log;
use crate::service::mcp::{JsonRPCMessage, McpError};

pub struct StdioTransport{
    pub child: Child,
    pub reader: BufReader<ChildStdout>,
    // pub err:BufReader<ChildStderr>,
    pub err_log_status:Arc<AtomicBool>,
    pub writer: ChildStdin,
}

impl StdioTransport {
    pub fn new<'a>(cmd:&str, args:impl IntoIterator<Item=&'a str>) -> anyhow::Result<Self> {
        let mut command = Command::new(cmd);
        for i in args.into_iter() {
            command.arg(i);
        }
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.kill_on_drop(true);
        let mut child = command.spawn()?;
        let reader = BufReader::new(child.stdout.take().unwrap());
        let writer = child.stdin.take().unwrap();
        let err_log_status = Arc::new(AtomicBool::new(true));
        Self { child, reader, writer,err_log_status }.ok()
    }

    async fn read_start_info(&mut self)->String{
        let mut lines = String::new();
        while let Ok(s) = self.read_line(Duration::from_secs(3)).await{
            lines.push_str(&s);
            lines.push('\n');
        }
        lines
    }
    pub fn enable_err_log<S:Into<String>>(&mut self,prefix:S){
        let mut err = BufReader::new(self.child.stderr.take().unwrap());
        let prefix = prefix.into();
        self.err_log_status.store(true,Ordering::Relaxed);
        let status  = self.err_log_status.clone();
        tokio::spawn(async move {
            while status.load(Ordering::Relaxed) {
                let mut line = String::new();
                match err.read_line(&mut line).await {
                    Ok(o) => {
                        if o == 0 {
                            break
                        }
                        wd_log::log_info!("[StdioTransport.{prefix}] {line}");
                    },
                    Err(e) => {
                        wd_log::log_error_ln!("[StdioTransport.{prefix}] error: {e}");
                        break;
                    },
                }
            }
        });
    }
    pub async fn read_line(&mut self,timeout:Duration) -> Result<String, McpError> {
        let mut buf = String::new();
        match tokio::time::timeout(timeout, self.reader.read_line(&mut buf)).await{
            Ok(Ok(_)) => Ok(buf),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err(McpError::Timeout),
        }
    }
    pub async fn write_line(&mut self,data:Vec<u8>)->Result<(),McpError>{
        self.writer.write_all(&data).await?;Ok(())
    }
}

#[async_trait::async_trait]
impl super::Transport for StdioTransport {
    async fn send(&mut self, msg: &JsonRPCMessage) -> Result<(), McpError> {
        let mut s = msg.to_json_str()?;
        wd_log::log_info_ln!("send: {}",s);
        s.push_str("\n");
        self.write_line(s.into_bytes()).await
    }

    async fn receive(&mut self,timeout:Duration) -> Result<JsonRPCMessage, McpError> {
        let s = self.read_line(timeout).await?;
        wd_log::log_info_ln!("receive: {}",s);
        JsonRPCMessage::from_json_str(s.as_str())
    }

    async fn close(&mut self) -> Result<(), McpError> {
        self.child.kill().await?;
        self.err_log_status.store(false, Ordering::Relaxed);
        Ok(())
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        self.err_log_status.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests{
    use crate::service::mcp::{JsonRPCMessageReq, RequestId, Transport, JSONRPC_VERSION_V2};
    use super::*;

    #[tokio::test]
    async fn test_stdio_transport() {
        let mut file_system = StdioTransport::new("npx", ["-y","@modelcontextprotocol/server-filesystem","~/project/work/other/mcp"]).unwrap();
        file_system.enable_err_log("file_system");
        // let start_info = file_system.read_start_info().await;
        // wd_log::log_info_ln!("start info: {}",start_info);
        // let log = file_system.read_log(Duration::from_secs(3)).await.unwrap();
        // wd_log::log_info_ln!("start log: {}",log);
        let init_msg = JsonRPCMessage::Req(JsonRPCMessageReq{
            jsonrpc: JSONRPC_VERSION_V2.to_string(),
            id: RequestId::Number(1),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities":{},
                "clientInfo": {
                    "name": "ExampleClient",
                    "title": "Example Client Display Name",
                    "version": "1.0.0"
                }
            }))
        });
        file_system.send(&init_msg).await.unwrap();
        let init_resp = file_system.receive(Duration::from_secs(3)).await.unwrap();
        wd_log::log_info_ln!("init resp: {:?}",init_resp);
        drop(file_system);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}