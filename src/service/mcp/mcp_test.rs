

#[cfg(test)]
mod test {
    use mcpr::client::Client;
    use mcpr::error::MCPError;
    use mcpr::schema::JSONRPCRequest;
    use mcpr::transport::stdio::StdioTransport;
    use serde_json::Value;

    #[tokio::test]
    async fn test_mcp_service() {
        let transport = StdioTransport::new();
        
        let mut client = Client::new(transport);
        
        if let Some(s) = wd_log::res_error_ln!(client.initialize()){
            let s = serde_json::to_string_pretty(&s).unwrap();
            wd_log::log_info_ln!("init successful: {}",s);
        };
    }
}