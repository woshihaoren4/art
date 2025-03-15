#[cfg(test)]
mod test{
    use std::time::Duration;
    use art::core::{Ctx, Engine, EngineRT, MapServiceLoader, Output, Service, ServiceEntity};
    use art::plan::dag::{DAGNode, DAG};

    async fn function_log_service_impl(_c:Ctx,se:ServiceEntity)->anyhow::Result<Output>{
        wd_log::log_field("ServiceEntity",se).info("function_log_service_impl:");
        Ok(Output::new("function_log_service_impl".to_string()))
    }
    async fn function_sleep_service_impl(_c:Ctx,_se:ServiceEntity)->anyhow::Result<Output>{
        wd_log::log_field("sleep","1s").info("function_sleep_service_impl:");
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(Output::new("function_sleep_service_impl".to_string()))
    }

    pub struct ServiceStruct{
        name:String,
    }

    impl ServiceStruct{
        pub fn new(name:&str)->Self{
            ServiceStruct{name:name.to_string()}
        }
    }

    #[async_trait::async_trait]
    impl Service for ServiceStruct{
        async fn call(&self, _ctx: Ctx, se: ServiceEntity) -> anyhow::Result<Output> {
            wd_log::log_field("ServiceEntity",se).field("Name",self.name.as_str()).info("function_log_service_impl:");
            Ok(Output::new(format!("【{}】",self.name)))
        }
    }

    async fn show_plan_start_middle(c:Ctx)->anyhow::Result<()>{
        c.deref_mut_plan(|p|{
            let info = p.show_plan();
            println!("plan->{}",info);
        });
        Ok(())
    }

    async fn show_node_name_middle(c:Ctx,se:ServiceEntity)->anyhow::Result<Output>{
        wd_log::log_field("node_name",se.node_name.as_str()).info("show_node_name_middle");
        c.next(se).await
    }

    fn new_services_default()->Engine{
        EngineRT::default()
            .set_service_loader(MapServiceLoader::default()
                .register_service("log",function_log_service_impl)
                .register_service("sleep",function_sleep_service_impl)
                .register_service("start",ServiceStruct::new("start"))
                .register_service("end",ServiceStruct::new("end")))
            .append_start_callback(show_plan_start_middle)
            .append_service_middle(show_node_name_middle)
            .build()
    }

    #[tokio::test]
    async fn single_services_test(){
        let plan =DAG::default()
            .node(DAGNode::new("START").set_service_entity("start"))
            .nodes([("a","sleep"),("b","sleep")])
            .node(("END","end"))
            .edges([("START","a"),("START","b")])
            .edges([("a","END"),("b","END")])
            .check().unwrap();
        let out:String = new_services_default()
            .ctx(plan)
            .run("hello world").await.unwrap();
        assert_eq!(out.as_str(),"【end】");
    }
}