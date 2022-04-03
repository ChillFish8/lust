<p align="center">
  <img width="50%" src="https://user-images.githubusercontent.com/57491488/160932579-518e61b8-6a3d-4400-a46c-1cb93d461417.png" alt="Lust Logo">
</p>

<p align="center"> 
 <h2 align="center">🔥 Build your own image CDN system your way with lust.</h2>
</p>

Lust is an **auto-optimising image server**, designed for **high throughput** and **low latency** handling of images, *now that is lustful*.
Re-encode uploaded images into `png`, `jpeg`, `webp` or even into `gif` based formats! 

Resize them to your liking automatically with sizing presets, instantly create small,
medium and large variants with just a few line in a config file. *Now that's the spirit of lust*

And much more like caching, on the fly resizing and diffrent processing modes to name a few.
## Getting started

### Creating a config file
It's highly advised to take a look at some [example config files](/examples/configs) to get an idea
of what a general config file would look like.

Full documentation in markdown form can also be found [here](description.md), this is also
served directly by the server as part of the documentation ui endpoint.

### Installation
#### Building from Source
To building from source, just clone this repo via `git clone https://github.com/chillfish8/lust.git` and then run `cargo build --release`.

#### Installing via Cargo
You can install lust directly via cargo and the git flag:
```shell
cargo install lust --git https://github.com/ChillFish8/lust.git
```

#### Docker Images
Lust has a set of pre-built, optimised docker images ready to go. Just run it with
```shell
docker run -v "my_configs:/var/lust/" chillfish8/lust:latest --config-file "/var/lust/config.yaml"
```
*Note: Assuming there is a folder called `my_configs` with a `config.yaml` file in it.*

### After Installation
Once you're up and running navigate to `http://127.0.0.1:8000/ui` or `/ui` of what ever port your server is running on
to see the full OpenAPI docs.
 
## Caching
Lust makes use of a Least Recently Used in-memory cache which can be adjusted for your needs via the `cache_size` key in the configuration file. 
The larger the number the more images it will cache at once and vice versa. 
*NOTE: With bigger images this can create much higher RAM usage*

## Scaling
Lust's ability to scale is purely down to the backend you use, so it is worth noting that
the file system backend is only really designed for testing. For full scale deployment
consider using Scylla or a s3 compatible blob store to serve data from.

If your goal is high-end performance, Scylla DB will be the most performant by a large
margin, but this will come with a higher operating cost.

## Formats
Lust supports any of the following formats: 
- Png
- JPEG
- GIF
- Webp
 
Any uploaded images will be given a unique uuid and be re-encoded into all the other enabled formats in all presets. 
This is especially useful when you want to serve several variants of the same image with different formats.

You can also adjust this based on the processing mode, `aot`/*Ahead of time* encoding will follow the old
lust behavour by encoding and resizing each image at upload time.

`jit`/*Just in time* encoding will only resize and re-encode at request time, storing a base copy
of the file to generate new images. This can save on a considerable amount of CPU time and disk space
depending on your requirements.

Finally, we have the `realtime` encoder, this will only store an original copy like the `jit` encoder
but instead will never save the resized and encoded image, this does also enable the ability to
do on the fly resizing and is recommended for situations where you're not expecting to serve image
to the public network.
 
## Presets
The server can take several sizing presets which can be targeted via the `size` 
query parameter when getting an image. 
These presets will mean every image at upload time will be resized to 
fit the width and height bounds using the configured resizing filter 
(defaults to nearest neighbour).

Regardless of presets an `original` image is always stored and can be accessed via the `size=original` query.
The default preset when served without a `size` parameter can be set in the configuration file via `default_serving_preset` key.

## Data Efficiency
Lust's data storage efficiency is roughly the same as storing on a plain file system outside any 
system the database backend employs when storing the data.
