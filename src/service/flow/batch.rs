use std::ops::DerefMut;
use std::sync::Arc;
use serde_json::Value;
use tokio::sync::Mutex;
use wd_tools::{PFArc, PFErr};
use wd_tools::pool::ParallelPool;
use crate::core::{Ctx, JsonInput, JsonServiceExt, ServiceEntity};
use crate::service::ext::Obj;

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BatchCfg {
    pub inputs:Vec<Value>,
    pub batch_max: usize,
    pub service: String,
    pub format : Obj,
}
#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BatchResult{
    pub outputs:Vec<Value>
}


#[derive(Default, Debug)]
pub struct Batch {}

impl Batch {
    pub fn make_sub_input_from_format(format:Obj,index:usize,input:Value)->JsonInput{
        JsonInput::default()
            .set_default_json(format.into())
            .add_transform_value("input",input)
            .add_transform_value("index",index)
    }
    pub async fn set_error(me:&Arc<Mutex<Option<anyhow::Error>>>,i:usize,err:anyhow::Error){
        let mut lock = me.lock().await;
        lock.replace(anyhow::anyhow!("Batch[{i}] error={}",err));
    }
    pub async fn get_error(me:&Arc<Mutex<Option<anyhow::Error>>>)->Option<anyhow::Error>{
        let mut lock = me.lock().await;
        lock.take()
    }
    pub async fn set_output(ma:&Arc<Mutex<Vec<Value>>>,i:usize,out:Value){
        let mut lock = ma.lock().await;
        lock[i] = out;
    }
    pub async fn get_output(ma:&Arc<Mutex<Vec<Value>>>)->Vec<Value>{
        let mut lock = ma.lock().await;
        std::mem::replace(lock.deref_mut(),Vec::new())
    }
    // pub async fn call_service(){
    //
    // }
}

#[async_trait::async_trait]
impl JsonServiceExt<BatchCfg, BatchResult> for Batch {
    async fn call(&self, ctx: Ctx, mut cfg: BatchCfg, se: ServiceEntity) -> anyhow::Result<BatchResult> {
        let s = if let Some(s) = ctx.rt.load_service(cfg.service.as_str()).await {
            s
        }else{
            return anyhow::anyhow!("BatchCfg.call:service[{}] not found", cfg.service).err()
        };
        if cfg.batch_max<=0{
            wd_log::log_field("Batch.cfg.batch_max",0).warn("update max = 1");
            cfg.batch_max = 1
        }

        let pp = ParallelPool::new(cfg.batch_max);
        let result_list = vec![Value::Null;cfg.inputs.len()];
        let output = Mutex::new(result_list).arc();
        let err = Mutex::new(None).arc();

        for (i,e) in cfg.inputs.into_iter().enumerate() {
            let input = Self::make_sub_input_from_format(cfg.format.clone(), i, e);
            let se = ServiceEntity::new(input)
                // .set_service(s.clone())
                .set_node_name(format!("{}_{}",se.node_name,i))
                .set_service_name(cfg.service.clone());
            let s = s.clone();
            let ctx = ctx.clone();
            let aerr = err.clone();
            let output = output.clone();
            pp.launch(async move {
                let result = s.call(ctx.clone(),se).await;
                let out = match result {
                    Ok(o) => o,
                    Err(e) => {
                        return Self::set_error(&aerr,i,e).await;
                    }
                };
                let res = out.into::<Value>();
                match res {
                    Ok(v) => Self::set_output(&output,i,v).await,
                    Err(e) => Self::set_error(&aerr,i,e).await,
                };
            }).await;
            if let Some(e) = Self::get_error(&err).await {
                return Err(e);
            }
        }
        pp.wait_over().await;
        let output_list = Self::get_output(&output).await;

        Ok(BatchResult{ outputs: output_list })
    }
}

#[cfg(test)]
mod test{
    use serde_json::{json, Number, Value};
    use crate::core::{CtxSerdeExt, EngineRT, JsonInput, ServiceEntity};
    use crate::plan::graph::{Graph, GraphNode};
    use crate::service::ext::{ServiceLoaderWrap};

    #[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
    #[serde(default)]
    struct AddInOut{
        input : usize,
        index : usize,
        result : usize,
        length:usize,
    }
    #[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
    #[serde(default)]
    struct ListMake{
        default:usize,
        len : usize,
    }

    #[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
    #[serde(default)]
    pub struct BatchResult{
        pub outputs:Vec<usize>
    }

    //cargo test --lib service::flow::batch::test::test_batch -- --show-output  --nocapture
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_batch(){
        let rt = EngineRT::default()
            .set_service_loader(
                ServiceLoaderWrap::default()
                    .register_json_ext_service(
                "add",
                |_ctx, mut io:AddInOut, _se| async move {
                    io.result = io.input + io.index;
                    println!("[{}:{}] {} + {} = {}",io.length,io.index,io.input,io.index,io.result);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    Ok(Value::Number(Number::from(io.result)))
                })
                    .register_json_ext_service(
                        "list_make",
                        |_ctx, lm:ListMake, se:ServiceEntity| async move {
                            let list = vec![lm.default;lm.len];
                            println!("[{}]: list->{:?}",se.node_name,list);
                            Ok(list)
                    })
            )
            .build();

        let batch_cfg = json!({
            "batch_max": 3,
            "inputs":"${{list_make}}",
            "service":"add",
            "format":{
                "length":"${{start.len}}"
            }
        });

        let plan = Graph::default()
            .node(("start",r#"{"service_name":"start","config":{"transform_rule":{"len":{"quote":"len"}}}}"#))
            .node(GraphNode::new("list_make").set_service_entity_json("list_make",JsonInput::default().set_default_json(json!({"default":1,"len":"${{start.len}}"}))))
            .node(GraphNode::new("batch_add").set_service_entity_json("batch",JsonInput::default().set_default_json(batch_cfg)))
            .node(("end", r#"{"service_name":"end","config":{"transform_rule":{"outputs":{"quote":"batch_add.outputs"}}}}"#))
            .edges([("start","list_make"),("list_make","batch_add"),("batch_add","end")])
            .check()
            .unwrap();

        let res:BatchResult =rt.ctx(plan).serde_run(json!({
                "len":10,
            })).await.unwrap();
        println!("result:{:?}",res);
        assert_eq!(res.outputs.len(), 10);
        assert_eq!(res.outputs[0],1);
        assert_eq!(res.outputs[1],2);
        assert_eq!(res.outputs[9],10);
    }

}