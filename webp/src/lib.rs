use image::DynamicImage;
use libwebp_sys::*;
use std::fmt::{Debug, Formatter, Error};
use std::ops::{Deref, DerefMut};
use libwebp_sys::WebPPreset::WEBP_PRESET_DEFAULT;


/// Inits the global encoder config.
///
///     - quality:
///         This parameter is the amount of effort put into the
///         compression: 0 is the fastest but gives larger
///         files compared to the slowest, but best, 100.
///
///     - method:
///         The quality / speed trade-off (0=fast, 6=slower-better)
///
///     - threads:
///         The amount of threads to attempt to use in multi-threaded encoding.
///
///     - efficiency:
///         The desired efficiency level between 0 (fastest, lowest compression)
///         and 9 (slower, best compression). A good default level is '6',
///         providing a fair tradeoff between compression speed and final
///         compressed size.
pub fn init_lossy(lossless: bool, quality: f32, method: i32, threads: u32, efficiency: u32) {
    let cfg = WebPConfig {
        lossless: if lossless { 1 } else { 0 },
        quality,
        method,
        image_hint: WebPImageHint::WEBP_HINT_DEFAULT,
        target_size: 0,
        target_PSNR: 0.0,
        segments: 4,
        sns_strength: 0,
        filter_strength: 0,
        filter_sharpness: 0,
        filter_type: 0,
        autofilter: 0,
        alpha_compression: 1,
        alpha_filtering: 1,
        alpha_quality: 100,
        pass: 5,
        show_compressed: 1,
        preprocessing: 0,
        partitions: 0,
        partition_limit: 0,
        emulate_jpeg_size: 0,
        thread_level: threads as i32,
        low_memory: 0,
        near_lossless: 100,
        exact: 0,
        use_delta_palette: 0,
        use_sharp_yuv: 0,
        pad: [100, 100]
    };

    unsafe {
        let ptr = Box::into_raw(Box::from(cfg)) as *mut WebPConfig;
        if lossless {
            WebPConfigInitInternal(
                ptr,
                WEBP_PRESET_DEFAULT,
                quality,
                WEBP_ENCODER_ABI_VERSION,
            );
        } else {
            WebPConfigLosslessPreset (
                ptr,
                efficiency as i32,
            );
        }
    }
}

#[derive(Copy, Clone)]
pub enum PixelLayout {
    RGB,
    RGBA,
}

pub struct Encoder<'a>  {
    layout: PixelLayout,
    image: &'a [u8],
    width: u32,
    height: u32,
}

impl<'a>  Encoder<'a>  {
    /// Creates a new encoder from the given image.
    pub fn from_image(image: &'a DynamicImage) -> Self {
        match image {
            DynamicImage::ImageLuma8(_) => { unreachable!() }
            DynamicImage::ImageLumaA8(_) => { unreachable!() }
            DynamicImage::ImageRgb8(image) =>
                Self::from_rgb(
                    image.as_ref(),
                    image.width(),
                    image.height()
                ),
            DynamicImage::ImageRgba8(image) =>
                Self::from_rgba(
                    image.as_ref(),
                    image.width(),
                    image.height()
                ),
            DynamicImage::ImageBgr8(_) => { unreachable!() }
            DynamicImage::ImageBgra8(_) => { unreachable!() }
            _ => { unreachable!() }
        }
    }

    /// Creates a new encoder from the given image data in the RGB pixel layout.
    pub fn from_rgb(image: &'a [u8], width: u32, height: u32) -> Self {
        Self { image, width, height, layout: PixelLayout::RGB }
    }

    /// Creates a new encoder from the given image data in the RGBA pixel layout.
    pub fn from_rgba(image: &'a [u8], width: u32, height: u32) -> Self {
        Self { image, width, height, layout: PixelLayout::RGBA }
    }

    /// Encode the image with the given quality.
    /// The image quality must be between 0.0 and 100.0 inclusive for minimal
    /// and maximal quality respectively.
    pub fn encode(&self, quality: f32) -> WebPMemory {
        unsafe { encode(self.image, self.layout, self.width, self.height, quality) }
    }

    /// Encode the image losslessly.
    pub fn encode_lossless(&self) -> WebPMemory {
        unsafe { encode(self.image, self.layout, self.width, self.height, -1.0) }
    }
}


unsafe fn encode(
    image: &[u8],
    layout: PixelLayout,
    width: u32,
    height: u32,
    quality: f32,
) -> WebPMemory{


    let width = width as _;
    let height = height as _;
    let mut buffer = std::ptr::null_mut::<u8>();

    let len = match layout {
        PixelLayout::RGB if quality < 0.0 => {
            let stride = width * 3;
            WebPEncodeLosslessRGB(image.as_ptr(), width, height, stride, &mut buffer as *mut _)
        }
        PixelLayout::RGB => {
            let stride = width * 3;
            WebPEncodeRGB(image.as_ptr(), width, height, stride, quality, &mut buffer as *mut _)
        }
        PixelLayout::RGBA if quality < 0.0 => {
            let stride = width * 4;
            WebPEncodeLosslessRGBA(image.as_ptr(), width, height, stride, &mut buffer as *mut _)
        }
        PixelLayout::RGBA => {
            let stride = width * 4;
            WebPEncodeRGBA(image.as_ptr(), width, height, stride, quality, &mut buffer as *mut _)
        }
    };

    WebPMemory(buffer, len)
}


/// This struct represents a safe wrapper around memory owned by libwebp.
/// Its data contents can be accessed through the Deref and DerefMut traits.
pub struct WebPMemory(pub(crate) *mut u8, pub(crate) usize);

impl Debug for WebPMemory {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.debug_struct("WebpMemory").finish()
    }
}

impl Drop for WebPMemory {
    fn drop(&mut self) {
        unsafe {
            WebPFree(self.0 as _)
        }
    }
}

impl Deref for WebPMemory {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.0, self.1) }
    }
}

impl DerefMut for WebPMemory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.0, self.1) }
    }
}