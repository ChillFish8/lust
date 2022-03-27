use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::Deserialize;
use crate::config::ImageKind;

pub mod realtime;
pub mod aot;
pub mod jit;
mod register;

pub use register::{Pipeline, PipelineSelector};

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingMode {
    /// Images will be optimised and resized when they're
    /// requested and then stored.
    Jit,

    /// Images have all optimizations and resizing applied to them
    /// and stored at upload time.
    Aot,

    /// Only the original image will be stored, any optimisations will always
    /// be ran at request time and not stored.
    Realtime,
}

impl Default for ProcessingMode {
    fn default() -> Self {
        Self::Jit
    }
}

impl ProcessingMode {
    pub fn build_pipeline(&self) -> PipelineController {
        // Macro magic, ignore any type errors by the linter here.
        let selector = match self {
            Self::Jit => PipelineSelector::from(jit::JustInTimePipeline {}),
            Self::Aot => PipelineSelector::from(aot::AheadOfTimePipeline {}),
            Self::Realtime => PipelineSelector::from(realtime::RealtimePipeline {}),
        };

        PipelineController {
            inner: selector.into(),
        }
    }
}

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
    pub to_store: Vec<StoreEntry>,
}

/// The raw binary data of the image.
pub type StoreEntry = Vec<u8>;

#[derive(Clone)]
pub struct PipelineController {
    inner: Arc<register::PipelineSelector>,
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

    pub fn on_fetch(
        &self,
        kind: ImageKind,
        data: Vec<u8>,
    ) -> anyhow::Result<ExecutionResult> {
        let instant = Instant::now();
        let result = self.inner.on_fetch(kind, data)?;
        let execution_time = instant.elapsed();

        Ok(ExecutionResult { result, execution_time })
    }
}

