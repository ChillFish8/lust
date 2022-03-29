use std::io::Cursor;
use std::sync::Arc;
use bytes::Bytes;
use image::{DynamicImage, ImageFormat};
use crate::config::{ImageFormats, ImageKind};


pub struct EncodedImage {
    pub kind: ImageKind,
    pub buff: Bytes,
    pub sizing_id: u32,
}

pub fn encode_following_config(
    cfg: ImageFormats,
    kind: ImageKind,
    img: DynamicImage,
    sizing_id: u32,
) -> anyhow::Result<Vec<EncodedImage>> {
    let original_image = Arc::new(img);

    let webp_config = webp::config(
        cfg.webp_config.quality.is_none(),
        cfg.webp_config.quality.unwrap_or(50f32),
        cfg.webp_config.method.unwrap_or(4) as i32,
        cfg.webp_config.threading,
    );

    let (tx, rx) = crossbeam::channel::bounded(4);

    for variant in ImageKind::variants() {
        if cfg.is_enabled(*variant) && (kind != *variant) {
            let tx_local = tx.clone();
            let local = original_image.clone();
            rayon::spawn(move || {
                let result = encode_to(webp_config, &local, (*variant).into());
                tx_local
                    .send(result.map(|v| EncodedImage { kind: *variant, buff: v, sizing_id }))
                    .expect("Failed to respond to encoding request. Sender already closed.");
            });
        }
    }

    // Needed to prevent deadlock.
    drop(tx);

    let mut processed = vec![];
    while let Ok(encoded) = rx.recv() {
        processed.push(encoded);
    }

    let mut finished = processed
        .into_iter()
        .collect::<Result<Vec<EncodedImage>, _>>()?;

    finished.push(EncodedImage {
       kind,
       sizing_id,
       buff: Bytes::from(original_image.as_ref().as_bytes().to_vec()),
    });

    Ok(finished)
}


pub fn encode_once(
    webp_cfg: webp::WebPConfig,
    to: ImageKind,
    img: DynamicImage,
    sizing_id: u32,
) -> anyhow::Result<EncodedImage> {
    let (tx, rx) = crossbeam::channel::bounded(4);

    rayon::spawn(move || {
        let result = encode_to(webp_cfg, &img, to.into());
        tx.send(result.map(|v| EncodedImage { kind: to, buff: v, sizing_id }))
            .expect("Failed to respond to encoding request. Sender already closed.");
    });

    rx.recv()?
}


#[inline]
pub fn encode_to(webp_cfg: webp::WebPConfig, img: &DynamicImage, format: ImageFormat) -> anyhow::Result<Bytes> {
    if let ImageFormat::WebP = format {
        let webp_image = webp::Encoder::from_image(webp_cfg, img);
        let encoded = webp_image.encode();

        return Ok(Bytes::from(encoded?.to_vec()))
    }


    let mut buff = Cursor::new(Vec::new());
    img.write_to(&mut buff, format)?;
    Ok(Bytes::from(buff.into_inner()))
}