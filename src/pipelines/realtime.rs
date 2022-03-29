use bytes::Bytes;
use hashbrown::HashMap;
use image::load_from_memory_with_format;
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
                .map(|(key, cfg)| (crate::utils::crc_hash(key), *cfg))
                .collect(),
            formats: cfg.formats,
        }
    }
}

impl Pipeline for RealtimePipeline {
    fn on_upload(&self, kind: ImageKind, data: Vec<u8>) -> anyhow::Result<PipelineResult> {
        let webp_config = webp::config(
            self.formats.webp_config.quality.is_none(),
            self.formats.webp_config.quality.unwrap_or(50f32),
            self.formats.webp_config.method.unwrap_or(4) as i32,
            self.formats.webp_config.threading,
        );

        let img = load_from_memory_with_format(&data, kind.into())?;
        let img = processor::encoder::encode_once(webp_config, self.formats.original_image_store_format, img, 0)?;

        dbg!(&img.kind, kind, self.formats.original_image_store_format);

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
        let webp_config = webp::config(
            self.formats.webp_config.quality.is_none(),
            self.formats.webp_config.quality.unwrap_or(50f32),
            self.formats.webp_config.method.unwrap_or(4) as i32,
            self.formats.webp_config.threading,
        );

        let img = load_from_memory_with_format(&data, data_kind.into())?;
        let (img, sizing_id) = if sizing_id != 0 {
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
                (processor::resizer::resize(cfg, &img), sizing_id)
            } else {
                (img, 0)
            }
        } else {
            (img, 0)
        };

        let encoded = processor::encoder::encode_once(
            webp_config,
            desired_kind,
            img,
            sizing_id,
        )?;

        Ok(PipelineResult {
            response: Some(StoreEntry {
                kind: encoded.kind,
                data: encoded.buff,
                sizing_id: encoded.sizing_id,
            }),
            to_store: vec![]
        })
    }
}