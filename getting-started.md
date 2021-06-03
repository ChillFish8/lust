# Contents

- [Setting Up Lust](#initalising-a-configuration)
- [Changing Configuration](#configuration-guide)
- [Running Lust](#running-lust)
- [Uploading an Image](#uploading-images)
- [Requesting Images](#requesting-images)
- [Removing Images](#removing-images)
- [Listing Images](#listing-images)

# Initalising a Configuration
Lust requires a configuration file to exist always and has many manditory keys. Because of this there is a utility command `init` which can be used to generate default configuration files.

#### Usage:
`lust init --backend <backend>`

The backend can be set to any of the valid backends:
- `postgres` -> Covers PostgreSQl.
- `mysql` -> Covers MySQL and MariaDB.
- `sqlite` -> Covers Sqlite.
- `cassandra` -> Covers Cassandra and Scylla (v4 protocol) **WARNING: This is very beta in terms of performant configuration**

Once the file is generated you can change the configuration as you wish however, be careful not to remove keys.

# Configuration Guide
Lust comes with several configurable controls which may seem confusing to some at first, so here's a helpful list of keys and their respective purpose.

### Server Configuration

- `log_level` -> What level of logging is enabled, this can be any of: `info` - `debug` - `error` - `off`
- `base_data_path` -> The base path images are served from. **This cannot be `admin` due to being reserved.**
- `cache_size` -> The maximum amount of images to keep in cache at once, this is based of a LRU eviction strategy.
- `database_backend` -> The database specific configuration (See the database configuration section bellow.)
- `default_serving_format` -> The format served when no `format` query parameter is passed when requesting an image.
- `default_serving_preset` -> The default sizing preset to serve, this can be any preset or `original`.
- `formats` -> A set of format-boolean pairs toggling the enabled formats which will be saved and re-encoded e.g.<br/> ```{
    "gif": false,
    "jpeg": true,
    "png": true,
    "webp": true
  }```
- `host` -> The binding host e.g. `127.0.0.1`.
- `port` -> The binding port e.g. `7070`.
- `size_presets` -> A set of maps defining seperate size presets which will auto resize images (See the size presets configuration section bellow.)
- `webp_ratio` -> The ratio of **lossy compression** for webp images from `0.0` to `100.0` inclusive for minimal and maximal quality respectively. This can be set to `null` to put the encoder into **lossless compression** mode.

### Database Configuration
Lust supports any of the following backends:
- `postgres` -> Covers PostgreSQl.
- `mysql` -> Covers MySQL and MariaDB.
- `sqlite` -> Covers Sqlite.
- `cassandra` -> Covers Cassandra and Scylla (v4 protocol) **WARNING: This is very beta in terms of performant configuration**

When configuring a backend in the server config the format should look like:
```js
"database_backend": {
  "config": {
    // Backend Specific
  },
  "type": "<backend>"
}
```

Each backend has a specific configuration layout see bellow:

### SQL based databases (Sqlite, PostgreSQL, MySQL)
- `connection_uri` -> The direct connection URI e.g. `postgres://user:pass@localhost/postgres`.
- `pool_size` -> The maxium connection pool size.

### Cassandra 
- `clusters` -> An array of strings following the `"ip:port"` format, each cluster should be one ip per machine.
- `keyspace` -> A detailed specification of the keyspace replication as specified bellow.
- `user` -> The username to connect with.
- `password` -> The password to connect with.

#### Keyspace Spec
Currently only `SimpleStrategy` and `NetworkTopologyStrategy` is supported.

#### SimpleStrategy Example
```js
"keyspace": {
  "spec": {
    "replication_factor": 3
  },
  "strategy": "SimpleStrategy"
}
```

#### NetworkTopologyStrategy Example
```js
"keyspace": {
  "spec": [
    {"node_name": "DC1", "replication": 3},
    {"node_name": "DC2", "replication": 2}    
  ],
  "strategy": "NetworkTopologyStrategy"
}
```

### Size Preset Configuration
Each preset name must be unique hence requires being defined like a map.
Each preset has a `width` and `height` key defining the sizing of the image.

**An `original` preset always exists and contains the original image uploaded**

#### Example
```js
"size_presets": {
  "large": {
    "height": 128,
    "width": 128
  },
  "medium": {
    "height": 64,
    "width": 64
  },
  "small": {
    "height": 32,
    "width": 32
  }
}
```

# Running Lust

Once the configuration has been setup you can use the `run` command to start the server: `lust run`

# Uploading Images

Lust requires images be uploaded as a Base64 encoded image via a JSON POST request to `/admin/create/image`. The body should follow the following schema:

| Field    | Description                                                                                                                                | Required?               |
|----------|--------------------------------------------------------------------------------------------------------------------------------------------|-------------------------|
| `format`   | The format of the image e.g. `png`, `jpeg`, etc...                                                                                         | Yes                     |
| `data`     | The base64 encoded image data.                                                                                                             | Yes                     |
| `category` | The category to add the image to, this will make the image accessible from  `/<base>/:category/:id` rather than the default `/<base>/:id`. | No (Default: 'default') |

# Requesting Images
Lust will serve images on the base route given via the `base_data_path` field in the config file, lets say this is `/images`, we can request an uploaded image with this path e.g. `http://127.0.0.1:7070/images/394e7905-f501-4be8-902f-b8b7ea9d157a`. If the image exists in the default category the server will return the image in the format specified by the `default_serving_format` from the preset defined with `default_serving_preset` in the configuration.

Each image request can have the optional query parameters:
| Field    | Description                                                                               |
|----------|-------------------------------------------------------------------------------------------|
| `format` | Request a specific format of the image e.g. `webp`.                                       |
| `encode` | Encodes the image with standard base64 encoding and returns the image as a JSON response. |
| `preset` | Requests a specific preset of the image e.g. `original`.                                  |

# Removing Images
Images can be removed via the `/admin/delete/image/:id` endpoint via a DELETE request with a JSON body.
The id should be the file's given UUID, no category is required because it is always unique.
*NOTE: This endpoint will always return 200 OK if an image doesnt exist, this is just a behavour of querying the database without pre-checking if it exists.*

# Listing Images
Lust gives you the ability to list and order the results in the database. **WARNING: Cassandra backends can regularly run into TimeoutErrors due to the nature of this request.**

Listing images be can accessed via the `/admin/list` endpoint and expects a POST request with a JSON body, all entries are chunked into 'pages' of `50` items per page.

An example body would look like:
```js
{
    "page": 1,
    "filter": {
        "filter_type": "category", // This can be any of 'all', 'category', 'creationdate'
        "with_value": "default" // Only required when using the 'category' or 'creationdate' filters.
    },
    "order": "creationdate" // Can be either of creationdate' or 'totalsize'.
}
```

*NOTE: The Cassandra backends will ignore the `order` field due to CQL limitations, all values with be in creation date order instead*

