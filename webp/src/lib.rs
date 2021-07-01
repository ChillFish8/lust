use std::fmt::{Debug, Formatter, Error};
use std::ops::{Deref, DerefMut};

use image::DynamicImage;
use once_cell::sync::OnceCell;

use libwebp_sys::*;
use libwebp_sys::WebPEncodingError::VP8_ENC_OK;
use libwebp_sys::WebPPreset::WEBP_PRESET_DEFAULT;


static CONFIG: OnceCell<WebPConfig> = OnceCell::new();

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
pub fn init_global(lossless: bool, quality: f32, method: i32, threads: u32) {
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
    
    let _ = CONFIG.set(cfg);
}

/// Picture is uninitialized.
pub fn empty_lossless_webp_picture() -> WebPPicture {
    WebPPicture {
        use_argb: 1,

        // YUV input
        colorspace: WebPEncCSP::WEBP_YUV420,
        width: 0,
        height: 0,
        y: std::ptr::null_mut(),
        u: std::ptr::null_mut(),
        v: std::ptr::null_mut(),
        y_stride: 0,
        uv_stride: 0,
        a: std::ptr::null_mut(),
        a_stride: 0,
        pad1: [0, 0],

        // ARGB input
        argb: std::ptr::null_mut(),
        argb_stride: 0,
        pad2: [
            0,
            0,
            0,
        ],

        // OUTPUT
        writer: None,
        custom_ptr: std::ptr::null_mut(),
        extra_info_type: 0,
        extra_info: std::ptr::null_mut(),

        // STATS AND REPORTS
        stats: std::ptr::null_mut(),
        error_code: VP8_ENC_OK,
        progress_hook: None,
        user_data: std::ptr::null_mut(),

        // padding for later use
        pad3: [0, 0, 0],

        // Unused for now
        pad4: std::ptr::null_mut(),
        pad5: std::ptr::null_mut(),

        // padding for later use
        pad6: [0, 0, 0, 0, 0, 0, 0, 0],

        // PRIVATE FIELDS
        memory_: std::ptr::null_mut(),
        memory_argb_: std::ptr::null_mut(),
        pad7: [std::ptr::null_mut(), std::ptr::null_mut()],
    }
}

/// Picture is uninitialized.
pub fn empty_lossy_webp_picture() -> WebPPicture {
    let mut picture = empty_lossless_webp_picture();
    picture.use_argb = 0;
    picture
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

    /// Encode the image with the given global config.
    pub fn encode(&self) -> WebPMemory {
        unsafe { encode(self.image, self.layout, self.width, self.height) }
    }
}


unsafe fn encode(
    image: &[u8],
    layout: PixelLayout,
    width: u32,
    height: u32,
) -> WebPMemory{

    let cfg = CONFIG.get()
        .expect("config un-initialised.")
        .clone();

    let mut picture = if cfg.lossless == 1 {
        empty_lossless_webp_picture()
    } else {
        empty_lossy_webp_picture()
    };

    let buffer = std::ptr::null_mut::<u8>();
    let writer = WebPMemoryWriter {
        mem: buffer,
        size: 0,
        max_size: 0,
        pad: [0]
    };

    let cfg_ptr = Box::into_raw(Box::from(cfg));
    let picture_ptr = Box::into_raw(Box::from(picture));
    let writer_ptr = Box::into_raw(Box::from(writer));
    if WebPConfigInitInternal(cfg_ptr, WEBP_PRESET_DEFAULT, cfg.quality, WEBP_ENCODER_ABI_VERSION) == 0 {
        panic!("config init failed");
    };

    if WebPPictureInitInternal(picture_ptr, WEBP_ENCODER_ABI_VERSION) == 0 {
        panic!("picture init failed");
    }


    WebPMemoryWriterInit(writer_ptr);

    let width = width as _;
    let height = height as _;

    picture.width = width;
    picture.height = height;

    picture.writer = WebPWriterFunction::None;
    picture.custom_ptr = writer_ptr as *mut _;

    let ok = match layout {
        PixelLayout::RGB => {
             let stride = width * 3;
             WebPPictureImportRGB(picture_ptr, image.as_ptr(), stride)
        }
        PixelLayout::RGBA => {
             let stride = width * 4;
             WebPPictureImportRGBA(picture_ptr, image.as_ptr(), stride)
        }
    };
    println!("{}", ok);

    let ok = WebPEncode(cfg_ptr, picture_ptr);
    println!("{:?}", (*picture_ptr).error_code);
    WebPPictureFree(picture_ptr);
    if ok == 0 {
        WebPMemoryWriterClear(writer_ptr);
        panic!("fuck, memory error. at encoding.")
    }

    WebPMemory(writer.mem, writer.size)
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;

    fn ensure_global() {
        init_global(
            true,
            90.0,
            3,
            10,
        )
    }

    #[test]
    fn test_basic_sample_1() {
        let image = image::open("./test_samples/news.png")
            .expect("load image");
        ensure_global();

        let encoder = Encoder::from_image(&image);
        let memory = encoder.encode();
        let buffer = memory.as_ref();
        write("./news.webp", buffer)
            .expect("write image");
    }

    #[test]
    fn test_basic_sample_2() {
        let image = image::open("./test_samples/release.png")
            .expect("load image");
        ensure_global();


        let encoder = Encoder::from_image(&image);
        let memory = encoder.encode();
        let buffer = memory.as_ref();
        write("./release.webp", buffer)
            .expect("write image");
    }
}