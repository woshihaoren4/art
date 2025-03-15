pub mod core;
pub mod plan;
pub mod service;

#[cfg(test)]
mod test {
    use crate::core::{Ctx, EngineRT, MapServiceLoader, Output};
    use crate::plan::dag::DAG;

    #[tokio::test]
    async fn simple_test() {
        let rt = EngineRT::default()
            .set_service_loader(MapServiceLoader::default()
                .register_service(
                "sa",
                |_x, _c| async {
                    wd_log::log_field("service_a : info:", "hello").debug("this is a test service");
                    Ok(Output::new("a->success".to_string()))
                })
                .register_service(
                    "sb",
                    |_x,_c|async {
                        wd_log::log_field("service_b : info:", "world").debug("this is a test service");
                        Ok(Output::new("b->success".to_string()))
                    }
                )
            )
            .append_service_middle(|ctx:Ctx,se|{
                println!("执行一个service:{}",se);
                ctx.next(se)
            })
            .append_start_callback(|c:Ctx|async move{
                c.deref_mut_plan(|p|{
                    println!("plan->{}",p.show_plan());
                });
                Ok(())
            })
            .build();
        let res = rt.ctx(DAG::default().nodes([("a","sa"),("b","sb")]).edge("a","b").check().unwrap())
            .run::<_,String>("xxx").await.unwrap();
        assert_eq!(res.as_str(),"b->success");
        println!("simple_test success");
    }
}
