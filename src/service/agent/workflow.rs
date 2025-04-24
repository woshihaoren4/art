use crate::core::Plan;
use std::sync::Arc;
use wd_tools::sync::Am;

pub struct Workflow {
    pub plan: Arc<Am<Box<dyn Plan + Sync + 'static>>>,
}

// impl Service for Workflow {
//
// }
