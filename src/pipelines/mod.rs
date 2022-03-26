use std::time::{Duration, Instant};
use crate::config::ImageKind;

pub mod realtime;
pub mod aot;
pub mod jit;
mod register;

pub use register::Pipeline;


pub struct ExecutionResult {
    /// The result of a given pipeline after a given operation.
    pub result: PipelineResult,

    /// The time taken to execute the pipeline.
    pub execution_time: Duration,
}

pub struct PipelineResult {
    /// To be returned to the client in some form.
    pub response: Option<StoreEntry>,

    /// To be persisted to the given storage backend.
    pub to_store: Option<StoreEntry>,
}

pub struct StoreEntry {
    /// The datastore key.
    pub key: String,

    /// The raw binary data of the image.
    pub data: Vec<u8>,
}

pub struct PipelineController {
    inner: register::PipelineSelector,
}

impl PipelineController {
    pub fn on_upload(
        &self,
        kind: ImageKind,
        data: Vec<u8>,
    ) -> anyhow::Result<ExecutionResult> {
        let instant = Instant::now();
        let result = self.inner.on_upload(kind, data)?;
        let execution_time = instant.elapsed();

        Ok(ExecutionResult { result, execution_time })
    }
}

