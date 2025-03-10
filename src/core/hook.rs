use crate::core::{Ctx, Engine, NextPlan, Output, ServiceEntity};

#[async_trait::async_trait]
pub trait FlowCallback: Send {
    async fn call(&self, ctx: Ctx) -> anyhow::Result<()>;
}

impl Engine {
    pub async fn base_hook(ctx:Ctx,se:ServiceEntity)->anyhow::Result<Output>{
        let node = se.node_name.clone();
        let rt = ctx.rt.clone();
        //处理返回结果
        let out = ctx.clone().next(se).await?;
        let node_key = node.clone();
        ctx.clone().async_mut_metadata(|c|{
            c.vars.insert(node_key,out);
            //todo 状态检查
            async {()}
        }).await;

        //继续向下执行
        let no_plan_ctx = ctx.clone_no_plan();
        let next = ctx.clone().deref_mut_plan(|c|{
            c.next(no_plan_ctx,node.as_str())
        })?;
        let nodes = match next {
            NextPlan::Nodes(nodes) => nodes,
            NextPlan::End => {
                ctx.success().await;
                return Ok(Output::new(()));
            }
            NextPlan::Wait => {
                return Ok(Output::new(()));
            }
        };
        for i in nodes{
            Engine::call_service(ctx.clone(),rt.clone(),i).await;
        }
        Ok(Output::new(()))
    }
}