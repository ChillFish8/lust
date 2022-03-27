use bytes::Bytes;
use hashbrown::HashMap;
use crate::config::{BucketConfig, ImageFormats, ImageKind, ResizingConfig};
use crate::pipelines::{Pipeline, PipelineResult};

pub struct JustInTimePipeline {
    presets: HashMap<u32, ResizingConfig>,
    formats: ImageFormats,
}

impl JustInTimePipeline {
    pub fn new(cfg: &BucketConfig) -> Self {
        Self {
            presets: cfg.presets
                .iter()
                .map(|(key, cfg)| (crate::utils::crc_hash(key), cfg.clone()))
                .collect(),
            formats: cfg.formats,
        }
    }
}

impl Pipeline for JustInTimePipeline {
    fn on_upload(&self, kind: ImageKind, data: Vec<u8>) -> anyhow::Result<PipelineResult> {
        todo!()
    }

    fn on_fetch(
        &self,
        kind: ImageKind,
        data: Bytes,
        sizing_id: u32,
        _custom_size: Option<(u32, u32)>,
    ) -> anyhow::Result<PipelineResult> {
        todo!()
    }
}