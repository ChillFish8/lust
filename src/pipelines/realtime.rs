use bytes::Bytes;
use hashbrown::HashMap;
use crate::config::{BucketConfig, ImageFormats, ImageKind, ResizingConfig};
use crate::pipelines::{Pipeline, PipelineResult, StoreEntry};
use crate::processor;

pub struct RealtimePipeline {
    presets: HashMap<u32, ResizingConfig>,
    formats: ImageFormats,
}

impl RealtimePipeline {
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

impl Pipeline for RealtimePipeline {
    fn on_upload(&self, kind: ImageKind, data: Vec<u8>) -> anyhow::Result<PipelineResult> {
        let img = processor::encoder::encode_once(self.formats.original_image_store_format, kind, data.into())?;

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
        custom_size: Option<(u32, u32)>,
    ) -> anyhow::Result<PipelineResult> {
        let img = processor::encoder::encode_once(desired_kind, data_kind, data)?;

        let (buff, sizing_id) = if sizing_id != 0 {
            let maybe_resize = match self.presets.get(&sizing_id) {
                None => if let Some((width, height)) = custom_size {
                    Some((
                        ResizingConfig {
                            width,
                            height,
                            filter: Default::default()
                        },
                        crate::utils::crc_hash((width, height)),
                    ))
                } else {
                    None
                },
                other => other.map(|v| (*v, sizing_id)),
            };

            if let Some((cfg, sizing_id)) = maybe_resize {
                (processor::resizer::resize(cfg, img.kind, img.buff)?, sizing_id)
            } else {
                (img.buff, 0)
            }
        } else {
            (img.buff, 0)
        };

        Ok(PipelineResult {
            response: Some(StoreEntry {
                kind: img.kind,
                data: buff,
                sizing_id
            }),
            to_store: vec![]
        })
    }
}