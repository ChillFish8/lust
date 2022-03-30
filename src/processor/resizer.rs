use std::sync::Arc;
use bytes::Bytes;
use hashbrown::HashMap;
use image::{DynamicImage, load_from_memory_with_format};
use crate::config::{ImageKind, ResizingConfig};

pub struct ResizedImage {
    pub sizing_id: u32,
    pub img: DynamicImage,
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
            let img = resize(cfg, &local);
            local_tx
                .send(ResizedImage { sizing_id, img })
                .expect("Failed to respond to encoding request. Sender already closed.");
        });
    }

    // Needed to prevent deadlock.
    drop(tx);

    let mut finished = vec![ResizedImage {
       sizing_id: 0,
       img: original_image.as_ref().clone(),
    }];
    while let Ok(encoded) = rx.recv() {
        finished.push(encoded);
    }

    Ok(finished)
}

pub fn resize(cfg: ResizingConfig, img: &DynamicImage) -> DynamicImage {
    img.resize(cfg.width, cfg.height, cfg.filter.into())
}