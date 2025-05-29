use crate::core::{Ctx, JsonServiceExt, ServiceEntity};
use crate::plan::dag::DAG;
use crate::service::ext::Obj;
use serde_json::Value;
use wd_tools::PFErr;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowPlan {
    #[default]
    None,
    DAG(DAG),
    GRAPH,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WorkflowConfig {
    pub plan: WorkflowPlan,
    pub input: Obj,
}

pub struct Workflow {}

#[async_trait::async_trait]
impl JsonServiceExt<WorkflowConfig, Value> for Workflow {
    async fn call(
        &self,
        ctx: Ctx,
        cfg: WorkflowConfig,
        se: ServiceEntity,
    ) -> anyhow::Result<Value> {
        let input = cfg.input;
        match cfg.plan {
            WorkflowPlan::None => {
                return anyhow::anyhow!("[Workflow::{}] plan is nil", se.node_name).err()
            }
            WorkflowPlan::DAG(dag) => ctx.fork(dag),
            WorkflowPlan::GRAPH => {
                return anyhow::anyhow!("[Workflow::{}] not support graph", se.node_name).err()
            }
        }
        .run::<Value, _>(input.into())
        .await
    }
}

impl Workflow {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod test {
    use crate::core::{Ctx, CtxSerdeExt, EngineRT, MapServiceLoader, ServiceEntity};
    use crate::plan::dag::DAG;
    use crate::service::agent::{Workflow, WorkflowPlan};
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
                    .register_json_ext_service("workflow", Workflow::new()),
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

        let wdag = DAG::default().nodes([
            ("start",r#"{"service_name":"start","config":{"transform_rule":{"a":{"quote":"a"}}}}"#),
            ("add",r#"{"service_name":"add","config":{"transform_rule":{"a":{"quote":"start.a"},"b":{"value":1}}}}"#),
            ("end",r#"{"service_name":"end","config":{"none_quote_skip":true,"transform_rule":{"res":{"quote":"add.res"}}}}"#),
        ]).edges([("start", "add"), ("add", "end")])
            .check()
            .unwrap();
        let wplan = WorkflowPlan::DAG(wdag);
        let wplan = serde_json::to_string(&wplan).unwrap();

        let workflow_cfg = format!("{{\"service_name\":\"workflow\",\"config\":{{\"transform_rule\":{{\"plan\":{{\"value\":{}}},\"input.a\":{{\"quote\":\"add.res\"}}}}}}}}",wplan);

        println!("--->workflow config:\n{}\n<---", workflow_cfg);

        let plan = DAG::default().nodes([
            ("start",r#"{"service_name":"start","config":{"transform_rule":{"a":{"quote":"a"}}}}"#),
            ("add",r#"{"service_name":"add","config":{"transform_rule":{"a":{"quote":"start.a"},"b":{"value":1}}}}"#),
            ("workflow_add",workflow_cfg.as_str()),
            ("end",r#"{"service_name":"end","config":{"none_quote_skip":true,"transform_rule":{"res":{"quote":"workflow_add.res"}}}}"#),
        ]).edges([("start", "add"), ("add", "workflow_add"), ("workflow_add", "end")])
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
