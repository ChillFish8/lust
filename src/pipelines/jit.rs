use bytes::Bytes;
use hashbrown::HashMap;
use crate::config::{BucketConfig, ImageFormats, ImageKind, ResizingConfig};
use crate::pipelines::{Pipeline, PipelineResult, StoreEntry};
use crate::processor;

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
        let webp_config = webp::config(
            self.formats.webp_config.quality.is_none(),
            self.formats.webp_config.quality.unwrap_or(50f32),
            self.formats.webp_config.method.unwrap_or(4) as i32,
            self.formats.webp_config.threading,
        );
        let img = processor::encoder::encode_once(webp_config, self.formats.original_image_store_format, kind, data.into())?;

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
        let webp_config = webp::config(
            self.formats.webp_config.quality.is_none(),
            self.formats.webp_config.quality.unwrap_or(50f32),
            self.formats.webp_config.method.unwrap_or(4) as i32,
            self.formats.webp_config.threading,
        );
        let img = processor::encoder::encode_once(webp_config, desired_kind, data_kind, data)?;

        let (buff, sizing_id) = if sizing_id != 0 {
            if let Some(cfg) = self.presets.get(&sizing_id) {
                (processor::resizer::resize(*cfg, img.kind, img.buff)?, sizing_id)
            } else {
                (img.buff, 0)
            }
        } else {
            (img.buff, 0)
        };

        Ok(PipelineResult {
            response: Some(StoreEntry {
                kind: img.kind,
                data: buff.clone(),
                sizing_id
            }),
            to_store: vec![StoreEntry { kind: img.kind, data: buff, sizing_id }]
        })
    }
}