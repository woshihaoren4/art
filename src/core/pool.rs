use std::future::Future;
use std::pin::Pin;

#[async_trait::async_trait]
pub trait RuntimePool: Send {
    async fn push(&self, fut: Pin<Box<dyn Future<Output = ()> + Send>>) {
        tokio::spawn(async move {
            fut.await
            // if let Err(e) = fut.await {
            //     wd_log::log_field("error", e).error("RuntimePool.fut run failed");
            // }
        });
    }
}
#[derive(Default)]
pub struct TokioRuntimePool {}
#[async_trait::async_trait]
impl RuntimePool for TokioRuntimePool {}
