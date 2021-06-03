# Starting With Lust

## Contents

- [Setting Up Lust](#initalising-a-configuration)
- [Running Lust](#running-lust)

## Initalising a Configuration
Lust requires a configuration file to exist always and has many manditory keys. Because of this there is a utility command `init` which can be used to generate default configuration files.

#### Usage:
`lust init --backend <backend>`

The backend can be set to any of the valid backends:
- `postgres` -> Covers PostgreSQl.
- `mysql` -> Covers MySQL and MariaDB.
- `sqlite` -> Covers Sqlite.
- `cassandra` -> Covers Cassandra and Scylla (v4 protocol) **WARNING: This is very beta in terms of performant configuration**

Once the file is generated you can change the configuration as you wish however, be careful not to remove keys.

## Configuration Guide
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

## Running Lust

Once the configuration has been setup you can use the `run` command to start the server: `lust run`
