use std::sync::Arc;
use bytes::Bytes;
use hashbrown::HashMap;
use image::{DynamicImage, load_from_memory_with_format};
use crate::config::{ImageKind, ResizingConfig};

pub struct ResizedImage {
    pub sizing_id: u32,
    pub buff: Bytes,
}

pub fn resize_image_to_presets(
    presets: &HashMap<u32, ResizingConfig>,
    kind: ImageKind,
    data: Bytes,
) -> anyhow::Result<Vec<ResizedImage>> {
    let original_image = Arc::new(load_from_memory_with_format(data.as_ref(), kind.into())?);

    let (tx, rx) = crossbeam::channel::bounded(presets.len());
    for (sizing_id, cfg) in presets {
        let sizing_id = *sizing_id;
        let cfg = *cfg;
        let local_tx = tx.clone();
        let local = original_image.clone();
        rayon::spawn(move || {
            let sized = resize_to_cfg(cfg, &local);
            local_tx
                .send(ResizedImage { sizing_id, buff: sized })
                .expect("Failed to respond to encoding request. Sender already closed.");
        });
    }

    // Needed to prevent deadlock.
    drop(tx);

    let mut finished = vec![ResizedImage {
       sizing_id: 0,
       buff: data,
    }];
    while let Ok(encoded) = rx.recv() {
        finished.push(encoded);
    }

    Ok(finished)
}

pub fn resize(
    cfg: ResizingConfig,
    kind: ImageKind,
    data: Bytes,
) -> anyhow::Result<Bytes> {
    let original_image = load_from_memory_with_format(data.as_ref(), kind.into())?;
    Ok(resize_to_cfg(cfg, &original_image))
}

fn resize_to_cfg(cfg: ResizingConfig, img: &DynamicImage) -> Bytes {
    let img = img.resize(cfg.width, cfg.height, cfg.filter.into());
    Bytes::from(img.into_bytes())
}