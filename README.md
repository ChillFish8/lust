<p align="center">
  <img width="60%" src="https://github.com/ChillFish8/lust/blob/master/assets/logo.png" alt="Lust Logo">
</p>

#

## What is Lust?
Lust is a static image server designed to automatically convert uploaded image to several formats and preset sizes with scaling in mind.
 
Lust stores images via any of given database backends:
 
- Redis / KeyDB
- Cassandra / ScyllaDB
- PostgreSQL
- MySQL / MariaDB
- Sqlite (file / temp file only)

## Getting started

#### Installation
Currently you can build from source, pre-built binary setups will be provided later, building just requires the tradition `cargo build --release`.

#### After Installation
See the [getting started page](https://github.com/ChillFish8/lust/blob/master/getting-started.md) for more information after installation.
 
## Caching
Lust makes use of a Least Recently Used in-memory cache which can be adjusted for your needs via the `cache_size` key in the configuration file. 
The larger the number the more images it will cache at once and vice versa. 
*NOTE: With bigger images this can create much higher RAM usage*

## Scaling
Lust's ability to scale is purely down to the backend you use, something like SQLite will obviously suffer 
at any sort of scale and is meant only really for development purposes.
Personally I recommend PostgreSQL (leading to vertical scaling storage) or Scylla (Horizontally scaling storage) depending on your needs.
If you want a very small amount of cached images then Postgres will out perform Scylla considerably at random reads however, 
Scylla is far more suites to large scaling and distributed system as well as large amounts of writes.

Performance of each database generally doesn't matter too much due to the processing time of each image 
being more than the IO latency when adding images and the cache supporting reads, that being said if 
you have a lot of random inconsistent reads PostgreSQL will likely be the best, or 
if you want large distributed scaling Scylla will allow you to scale horizontally.

If you want the best of both worlds I would recommend looking at KeyDB (Redis) with disk persistence, when setup correctly this
can be an incredibly powerful setup.

## Formats
Lust supports any of the following formats: 
- Png
- JPEG
- GIF
- Webp
 
Any uploaded images will be given a unique uuid and be re-encoded into all the other enabled formats in all presets. 
This is especially useful when you want to serve several variants of the same image with different formats.
 
## Presets
The server can take several sizing presets which can be targeted via the `size` query parameter when getting an image. These presets will mean every image at upload time will be resized to fit the width and height bounds using the nearest approximation.

Regardless of presets an `original` image is always stored and can be accessed via the `size=original` query.
The default preset when served without a `sized` parameter can be set in the configuration file via `default_serving_preset` key.

## Webp Optimisation
Lust supports automatic webp encoding, by default it encodes with lossless compression but this can be changed via the `webp_quality` key in the configuration file
and should be a float from `0.0` to `100.0` with the quality of the image changing respectively.

## Base64 Support

Lust will serve given images / gifs as Base64 data the `encode` query parameter (`true`/`false`) this will return
a JSON response unlike the tradition raw response.

## Data Efficiency
Lust's data storage efficiency is roughly the same as storing on a plain file system outside of any system the database backend employs when storing the data.

For example lets upload an image: 
<p align="left">
  <img width="50%" src="https://github.com/ChillFish8/lust/blob/master/assets/news.png" alt="Medium image">
</p>

This image is about 91KB in size as a single image, If we upload this and convert to the 3 base image formats with some presets:

```json5
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
