use enum_dispatch::enum_dispatch;
use crate::config::ImageKind;
use crate::pipelines::PipelineResult;

use super::realtime::RealtimePipeline;
use super::aot::AheadOfTimePipeline;
use super::jit::JustInTimePipeline;

/// Pipelines are dynamically selected here.
///
/// This is not a Box<dyn Trait> due to this being rather
/// performance critical and this approach allows for more
/// room for the compiler to optimise.
#[allow(clippy::enum_variant_names)]
#[enum_dispatch(Pipeline)]
pub enum PipelineSelector {
    RealtimePipeline,
    AheadOfTimePipeline,
    JustInTimePipeline,
}

#[enum_dispatch]
pub trait Pipeline: Sync + Send + 'static {
    fn on_upload(&self, kind: ImageKind, data: Vec<u8>) -> anyhow::Result<PipelineResult>;

    fn on_fetch(&self, kind: ImageKind, data: Vec<u8>) -> anyhow::Result<PipelineResult>;
}