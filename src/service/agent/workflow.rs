use std::sync::Arc;
use wd_tools::sync::Am;
use crate::core::{Plan};

pub struct Workflow{
    pub plan : Arc<Am<Box<dyn Plan + Sync + 'static>>>,
}

// impl Service for Workflow {
//
// }