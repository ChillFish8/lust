use bytes::Bytes;
use hashbrown::HashMap;

use crate::config::{BucketConfig, ImageFormats, ImageKind, ResizingConfig};
use crate::pipelines::{Pipeline, PipelineResult};
use crate::processor;

pub struct AheadOfTimePipeline {
    presets: HashMap<u32, ResizingConfig>,
    formats: ImageFormats,
}

impl AheadOfTimePipeline {
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

impl Pipeline for AheadOfTimePipeline {
    fn on_upload(&self, kind: ImageKind, data: Vec<u8>) -> anyhow::Result<PipelineResult> {
        let encoded_images = processor::encoder::encode_following_config(
            self.formats,
            kind,
            Bytes::from(data),
        )?;

        todo!()
    }

    fn on_fetch(
        &self,
        kind: ImageKind,
        data: Vec<u8>,
        sizing_id: u32,
        _custom_size: Option<(u32, u32)>,
    ) -> anyhow::Result<PipelineResult> {
        todo!()
    }
}