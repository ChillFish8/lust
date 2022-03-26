use crate::config::ImageKind;
use crate::pipelines::{Pipeline, PipelineResult};

pub struct AheadOfTimePipeline;

impl Pipeline for AheadOfTimePipeline {
    fn on_upload(&self, kind: ImageKind, data: Vec<u8>) -> anyhow::Result<PipelineResult> {
        todo!()
    }

    fn on_fetch(&self, kind: ImageKind, data: Vec<u8>) -> anyhow::Result<PipelineResult> {
        todo!()
    }
}