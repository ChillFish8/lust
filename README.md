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
Currently you can build from source, re-built binary setups will be provided later, building just requires the tradition `cargo build --release`.

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

## Webp Optimisation
Lust supports webp automisation encoding, by default it encodes with lossless compression but this can be changed via the `webp_quality` key in the configuration file
and should be a float from `0.0` to `100.0` with the quality of the image changing respectively.

## Base64 Support

Lust will serve given images / gifs as Basse64 data both Gzip compressed and un-compressed via the `encode` query parameter (`true`/`false`) this will return
a JSON response unlike the tradition raw response.

## Data Efficiency
Lust's data storage efficiency is roughly the same as storing on a plain file system outside of any system the database backend employs when storing the data.

For example lets upload an image: 
<p align="left">
  <img width="50%" src="https://github.com/ChillFish8/lust/blob/master/assets/news.png" alt="Medium image">
</p>

This image is about 91KB in size as a single image, If we upload this and convert to the 3 base image formats with some presets:

```js
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

We can see with the `original` size totals around 200KB which is is fairly reasonable with zero compression PNG encoding and lossless webp formats.
