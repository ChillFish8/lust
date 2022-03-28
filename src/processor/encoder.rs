use std::io::Cursor;
use std::sync::Arc;
use bytes::Bytes;
use image::{DynamicImage, ImageFormat, load_from_memory_with_format};
use crate::config::{ImageFormats, ImageKind};


pub struct EncodedImage {
    pub kind: ImageKind,
    pub buff: Bytes,
}

pub fn encode_following_config(
    cfg: ImageFormats,
    kind: ImageKind,
    data: Bytes,
) -> anyhow::Result<Vec<EncodedImage>> {
    let original_image = Arc::new(load_from_memory_with_format(data.as_ref(), kind.into())?);
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
                    .send(result.map(|v| EncodedImage { kind: *variant, buff: v }))
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
       buff: data,
    });

    Ok(finished)
}


pub fn encode_once(
    webp_cfg: webp::WebPConfig,
    to: ImageKind,
    from: ImageKind,
    data: Bytes,
) -> anyhow::Result<EncodedImage> {
    let original_image = load_from_memory_with_format(data.as_ref(), from.into())?;

    let (tx, rx) = crossbeam::channel::bounded(4);

    let encoded = if from != to {
        rayon::spawn(move || {
            let result = encode_to(webp_cfg, &original_image, to.into());
            tx.send(result.map(|v| EncodedImage { kind: to, buff: v }))
                .expect("Failed to respond to encoding request. Sender already closed.");
        });

        rx.recv()??
    } else {
        EncodedImage {
            kind: to,
            buff: data,
        }
    };

    Ok(encoded)
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