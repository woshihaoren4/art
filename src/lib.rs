pub mod core;
pub mod plan;
pub mod service;
mod utils;

#[cfg(test)]
mod test {
    use crate::core::{Ctx, CtxSerdeExt, EngineRT, JsonServiceExt, ServiceEntity};
    use crate::plan::graph::Graph;
    use crate::service::ext::ServiceLoaderWrap;
    use serde::{Deserialize, Serialize};
    use wd_tools::PFOk;

    #[derive(Serialize, Deserialize, Default)]
    struct ChatModelReq {
        query: String,
        name: String,
    }
    #[derive(Serialize, Deserialize, Default)]
    struct ChatModelResp {
        answer: String,
    }
    #[derive(Default)]
    struct ChatModel {}

    #[async_trait::async_trait]
    impl JsonServiceExt<ChatModelReq, ChatModelResp> for ChatModel {
        async fn call(
            &self,
            _ctx: Ctx,
            input: ChatModelReq,
            _se: ServiceEntity,
        ) -> anyhow::Result<ChatModelResp> {
            wd_log::log_field("query", input.query.as_str()).info("ChatModel->a new request");
            ChatModelResp {
                answer: format!("{}:{}", input.name, input.query),
            }
            .ok()
        }
    }
    #[derive(Debug, Serialize, Deserialize)]
    struct TaskResponse {
        answer1: String,
        answer2: String,
    }

    #[tokio::test]
    async fn simple_test() {
        let rt = EngineRT::default()
            .set_service_loader(
                ServiceLoaderWrap::default()
                    .register_json_ext_service("chat_model", ChatModel::default()),
            )
            .append_service_middle(|ctx: Ctx, se| {
                println!("执行一个service:{}", se);
                ctx.next(se)
            })
            .append_start_callback(|c: Ctx| async move {
                c.deref_mut_plan(|p| {
                    println!("plan->{}", p.show_plan());
                });
                Ok(())
            })
            .build();
        let plan = Graph::default().nodes([
            ("start",r#"{"service_name":"start","config":{"transform_rule":{"query":{"quote":"query"}}}}"#),
            ("m1",r#"{"service_name":"chat_model","config":{"transform_rule":{"query":{"quote":"start.query"},"name":{"value":"chat_mode_1"}}}}"#),
            ("m2",r#"{"service_name":"chat_model","config":{"transform_rule":{"query":{"quote":"start.query"},"name":{"value":"chat_mode_2"}}}}"#),
            ("end",r#"{"service_name":"end","config":{"none_quote_skip":true,"transform_rule":{"answer1":{"quote":"m1.answer"},"answer2":{"quote":"m2.answer"}}}}"#),
        ]).edges([("start", "m1"), ("start", "m2")])
            .edges([("m1", "end"), ("m2", "end")])
            .check()
            .unwrap();
        let res = rt
            .ctx(plan)
            .serde_run::<_, TaskResponse>(serde_json::json!({
                "query":"this is a test input."
            }))
            .await
            .unwrap();
        println!("resp->{:?}", res);
    }
}
