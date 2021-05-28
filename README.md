<p align="center">
  <img width="60%" src="https://github.com/ChillFish8/lust/blob/master/assets/logo.png" alt="Lust Logo">
</p>

#

## What is Lust?
Lust is a static image server designed to automatically convert uploaded image to several formats and preset sizes with automatic compression optimizations along with scaling in mind.
 
Lust stores images via any of given database backends:
 
- Cassandra / ScyllaDB
- PostgreSQL
- MySQL / MariaDB
- Sqlite (file / temp file only)

## Getting started

#### Installation
You can download a pre-made binary from the given github releases, binaries exist for both linux and windows (x64 and x86) or you can build from source.
Building just requires the tradition `cargo build --release`.

#### After Instalition
See the [getting started page](https://github.com/ChillFish8/lust/blob/master/getting-started.md) for more information after installation.
 
## Formats
Lust supports any of the following formats: 
- Png
- JPEG
- GIF
- Webp
 
Any uploaded images will be given a unqiue uuid and be re-encoded into all the other enabled formats in all presets. This is especially useful when you want to serve several varients of the same image with diffrent formats.
 
## Presets
The server can take several sizing presets which can be targetted via the `size` query parameter when getting an image. These presets will mean every image at upload time will be resized to fit the width and height bounds using the nearest approximation.

Regardless of presets an `original` image is always stored and can be accessed via the `size=original` query.
The default preset when served without a `sized` parameter can be set in the configuration file via `default_serving_preset` key.

## Compression
Lust supports Gzip compression when uploading image content and automatically creates Gzipped data varients which are what get stored in the database,
in the case of most modern webbrowsers, they will accept gzip as a method of compression. If this is the case the server will serve the pre-compressed data without any CPU overhead.

This behavour can be toggled using the `serve_compression_mode` ket with a value being any of the following:

| **Compression Mode** | **Description**                                                                                                                                                                             |
|----------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `always `              | The server will always serve the compressed version of the image regardless  of headers or query paremeters.                                                                                |
| `never`                | The server will never serve the compressed version of the image regardless of headers or query parameters.<br/> **WARNING**: This can create much higher CPU loads compared to other modes. |
| `auto`              | The server will try to send the compressed version when it can either via the `compress` query parameter or via the `Accept-Encoding` header. (Recommended)                                 |


## Base64 Support

Lust will serve given images / gifs as Basse64 data both Gzip compressed and un-compressed via the `encode` query parameter (`true`/`false`) this will return
a JSON response unlike the tradition raw response.

## Data Efficiency
Lust's use of compression for storage allows it to achieve relatively good data store despite creating several versions of the same image.

For example lets upload an image: 
<p align="left">
  <img width="50%" src="https://github.com/ChillFish8/lust/blob/master/assets/news.png" alt="Medium image">
</p>

This image is about 91KB in size as a single image, If we upload this and convert to the 3 base image formats with some presets:

```json
{
  'data': {
    'file_id': 'ccbe2207-8629-4938-9da9-3f75706f9b4e',
    'formats': {
      'large': {     // Resized to 128px x 128px
        'jpeg': 3460,
        'png': 5292,
        'webp': 3006
      },
      'medium': {   // Resized to 64px x 64px
        'jpeg': 1543, 
        'png': 1738, 
        'webp': 1022
      },
      'original': {   
        'jpeg': 42846,
        'png': 103672, 
        'webp': 53982
      },
      'small': {    // Resized to 32px x 32px
        'jpeg': 912, 
        'png': 629, 
        'webp': 354
      }
  },
  'status': 200
}
```

We can see with the `original` size totals around 200KB which is is fairly reasonable with 0 compression PNG encoding and lossless webp formats.
