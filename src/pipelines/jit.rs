use bytes::Bytes;
use hashbrown::HashMap;
use crate::config::{BucketConfig, ImageFormats, ImageKind, ResizingConfig};
use crate::pipelines::{Pipeline, PipelineResult, StoreEntry};
use crate::processor::encoder::encode_once;

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
        let img = encode_once(self.formats.original_image_store_format, kind, data.into())?;

        Ok(PipelineResult {
            response: None,
            to_store: vec![StoreEntry { kind: img.kind, data: img.buff, sizing_id: 0 }],
        })
    }

    fn on_fetch(
        &self,
        desired_kind: ImageKind,
        data_kind: ImageKind,
        data: Bytes,
        sizing_id: u32,
        _custom_size: Option<(u32, u32)>,
    ) -> anyhow::Result<PipelineResult> {
        todo!()
    }
}