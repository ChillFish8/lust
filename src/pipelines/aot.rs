use anyhow::anyhow;
use bytes::Bytes;
use hashbrown::HashMap;

use crate::config::{BucketConfig, ImageFormats, ImageKind, ResizingConfig};
use crate::pipelines::{Pipeline, PipelineResult, StoreEntry};
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

        let mut to_store = vec![];
        for encoded in encoded_images {
            to_store.push(StoreEntry { kind: encoded.kind, data: encoded.buff.clone(), sizing_id: 0 });

            let resized = processor::resizer::resize_image_to_presets(&self.presets, encoded.kind, encoded.buff)?;
            to_store.extend(resized.into_iter().map(|v| StoreEntry { kind: encoded.kind, sizing_id: v.sizing_id, data: v.buff }));
        }

        Ok(PipelineResult {
            response: None,
            to_store,
        })
    }

    fn on_fetch(
        &self,
        _desired_kind: ImageKind,
        data_kind: ImageKind,
        data: Bytes,
        sizing_id: u32,
        _custom_size: Option<(u32, u32)>,
    ) -> anyhow::Result<PipelineResult> {
        Ok(PipelineResult {
            response: Some(StoreEntry {
                data,
                sizing_id,
                kind: data_kind,
            }),
            to_store: vec![],
        })
    }
}