use std::io::{Cursor, Seek, Write};
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
    let original: ImageFormat = kind.into();
    let original_image = Arc::new(load_from_memory_with_format(data.as_ref(), original)?);

    let (tx, rx) = crossbeam::channel::bounded(4);

    for variant in ImageKind::variants() {
        if cfg.is_enabled(*variant) && (kind != *variant) {
            let tx_local = tx.clone();
            let local = original_image.clone();
            rayon::spawn(move || {
                let result = encode_to(&local, ImageFormat::Png);
                tx_local.send(result.map(|v| EncodedImage { kind: ImageKind::Png, buff: v }));
            });
        }
    }

    let mut finished = vec![EncodedImage {
       kind,
       buff: data,
    }];
    while let Ok(encoded) = rx.recv() {
        finished.push(encoded?);
    }

    Ok(finished)
}


pub fn encode_once(
    cfg: ImageFormats,
    to: ImageKind,
    from: ImageKind,
    data: Bytes,
) -> anyhow::Result<EncodedImage> {
    let original: ImageFormat = from.into();
    let original_image = load_from_memory_with_format(data.as_ref(), original)?;

    let (tx, rx) = crossbeam::channel::bounded(4);

    let encoded = if from != to {
        rayon::spawn(move || {
            let result = encode_to(&original_image, ImageFormat::Png);
            tx.send(result.map(|v| EncodedImage { kind: ImageKind::Png, buff: v }));
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
pub fn encode_to(img: &DynamicImage, format: ImageFormat) -> anyhow::Result<Bytes> {
    let mut buff = Cursor::new(Vec::new());
    img.write_to(&mut buff, format)?;
    Ok(Bytes::from(buff.into_inner()))
}