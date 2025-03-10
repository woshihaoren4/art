pub mod core;
pub mod plan;
pub mod service;

#[cfg(test)]
mod test {
    use crate::core::{EngineRT, MapServiceLoader, Output};

    #[test]
    fn simple_test() {
        let rt = EngineRT::default()
            .set_service_loader(MapServiceLoader::default().register_service(
                "log",
                |_x, _c| async {
                    wd_log::log_field("info:", "hello world").debug("this is a test service");
                    Ok(Output::new(()))
                },
            ))
            .build();
    }
}
