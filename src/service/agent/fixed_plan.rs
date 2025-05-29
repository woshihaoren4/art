use crate::core::{Ctx, JsonServiceExt, Plan, ServiceEntity};
use crate::service::ext::Obj;
use serde_json::Value;

pub struct FixedPlan<T> {
    pub plan: T,
}

#[async_trait::async_trait]
impl<T: Plan + Clone + Sync + 'static> JsonServiceExt<Obj, Value> for FixedPlan<T> {
    async fn call(&self, ctx: Ctx, input: Obj, _se: ServiceEntity) -> anyhow::Result<Value> {
        ctx.fork(self.plan.clone())
            .run::<Value, _>(input.into())
            .await
    }
}

impl<T> FixedPlan<T> {
    pub fn new(plan: T) -> Self {
        Self { plan }
    }
}

#[cfg(test)]
mod test {
    use crate::core::{Ctx, CtxSerdeExt, EngineRT, MapServiceLoader, ServiceEntity};
    use crate::plan::dag::DAG;
    use crate::service::agent::FixedPlan;
    use crate::service::flow::{End, Start};
    use serde::{Deserialize, Serialize};

    #[tokio::test]
    async fn test_workflow() {
        #[derive(Debug, Default, Clone, Serialize, Deserialize)]
        struct AddRequest {
            a: usize,
            b: usize,
        }
        #[derive(Debug, Default, Clone, Serialize, Deserialize)]
        struct AddResponse {
            res: usize,
        }
        let plan = DAG::default().nodes([
            ("start",r#"{"service_name":"start","config":{"transform_rule":{"a":{"quote":"a"}}}}"#),
            ("add",r#"{"service_name":"add","config":{"transform_rule":{"a":{"quote":"start.a"},"b":{"value":1}}}}"#),
            ("end",r#"{"service_name":"end","config":{"none_quote_skip":true,"transform_rule":{"res":{"quote":"add.res"}}}}"#),
        ]).edges([("start", "add"), ("add", "end")])
            .check()
            .unwrap();
        let add_1_workflow = FixedPlan::new(plan);
        let rt = EngineRT::default()
            .set_service_loader(
                MapServiceLoader::default()
                    .register_json_ext_service("start", Start {})
                    .register_json_ext_service("end", End {})
                    .register_json_ext_service(
                        "add",
                        |_ctx: Ctx, input: AddRequest, _se: ServiceEntity| async move {
                            Ok(AddResponse {
                                res: input.a + input.b,
                            })
                        },
                    )
                    .register_json_ext_service("add_1_workflow", add_1_workflow),
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

        let plan = DAG::default().nodes([
            ("start",r#"{"service_name":"start","config":{"transform_rule":{"a":{"quote":"a"}}}}"#),
            ("add",r#"{"service_name":"add","config":{"transform_rule":{"a":{"quote":"start.a"},"b":{"value":1}}}}"#),
            ("add_1_workflow",r#"{"service_name":"add_1_workflow","config":{"transform_rule":{"a":{"quote":"add.res"}}}}"#),
            ("end",r#"{"service_name":"end","config":{"none_quote_skip":true,"transform_rule":{"res":{"quote":"add_1_workflow.res"}}}}"#),
        ]).edges([("start", "add"), ("add", "add_1_workflow"), ("add_1_workflow", "end")])
            .check()
            .unwrap();
        let res = rt
            .ctx(plan)
            .serde_run::<_, AddResponse>(serde_json::json!({
                "a":1
            }))
            .await
            .unwrap();
        println!("resp->{:?}", res);
    }
}
