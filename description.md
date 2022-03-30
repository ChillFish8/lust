# The Lust 2 documentation

Welcome to the Lust 2 API documentation!

This contains section contains the configuration documentation for running and building your system.

## CLI
```shell
lust 2.0.0                                                                                        
Harrison Burt <hburt2003@gmail.com>                                                               
A fast, auto-optimising image server designed for multiple backends with throughput and latency in
mind.                                                                                             
                                                                                                  
USAGE:                                                                                            
    lust.exe [OPTIONS] --config-file <CONFIG_FILE>                                                
                                                                                                  
OPTIONS:                                                                                          
        --config-file <CONFIG_FILE>                                                               
            The file path to a given config file.                                                 
                                                                                                  
            This can be either a JSON formatted config or YAML.                                   
                                                                                                  
            [env: CONFIG_FILE=]                                                                   

    -d, --docs-url <DOCS_URL>
            The external URL that would be used to access the server if applicable.

            This only affects the documentation.

            [env: DOCS_URL=]

    -h, --host <HOST>
            The binding host address of the server

            [env: HOST=]
            [default: 127.0.0.1]

        --help
            Print help information

        --log-level <LOG_LEVEL>
            [env: LOG_LEVEL=]
            [default: info]

    -p, --port <PORT>
            [env: PORT=]
            [default: 8000]

    -V, --version
            Print version information
```

## Config File
This is a demo config file outlining and explain each configuration key.

*Note: This is in the YAML format, but an equivalent in JSON is also supported*

```yaml 
global_cache:
    # We cache upto 1GB's worth of the most recently used images.
    # Like the bucket cache a max_images limit can also be applied
    # but not used in tandom with the max_capacity limit.
    # If this is `null`/unset then no caching is performed.
    max_capacity: 1024  
    
# The *global* max upload size allowed in KB.
# 
# This takes precedence over bucket level limits.
max_upload_size: 4096  # 4MB

# The global max concurrency.
# 
# This takes precedence over bucket level limits.
max_concurrency: 500

# A custom base path to serve images out of.
# This gets appended to the `v1` route and must start with a `/`
base_serving_path: "/images"

backend:
    filesystem:  # Can be any of 'scylla', 'filesystem' or 'blobstorage'
    
        # Attributes are specific to the selectect backend.    
        # For the filesystem backend only the `directory` arguement is required
        # and is the base directory for images to be stored.
        directory: "/data"  
        
        # scylla attributes
        #
        # nodes:      # A list of known nodes.
        #     - "127.0.0.1:9042"  
        # keyspace: lust  # The keyspace must be created ahead of time.
        # username: 'my-user'  # Optional
        # password: 'my-pass'  # Optional
        # table: 'images'  # Optional, defaults to `lust_images`
        
        # blobstore attributes
        # 
        # This also requires `AWS_SECRET_ACCESS_KEY` and `AWS_ACCESS_KEY_ID`
        # environment varibles for auth.
        # name: "my-bucket"
        # region: "my-s3-region"
        # endpoint: "https://s3.eu2.my-endpoint.com"
        # store_publc: false  # If true, images are uploaded with acl: `public-read`.
        
buckets:
    my-profile-pictures:
        mode: jit   # 'jit', 'aot' or 'realtime' are allowed.
        
        formats:                 
          png: true  # Disable PNG encoding.
          jpeg: true  # Enable JPEG encoding.
          webp: true  # Enable WebP encoding.
          gif: false  # Disable GIF encoding.
    
          # The format to store the original image in.
          # This will be used by the 'jit' and 'realtime' encoders
          # when a image is requested as a base.
          # This probably does not want to be a lossy format.
          original_image_store_format: jpeg 
    
          webp_config:
            # This parameter is the amount of effort put into the
            # compression: 0 is the fastest but gives larger
            # files compared to the slowest, but best, 100.
            #
            # If set to `null` this will enabled lossless encoding.
            quality: 80       # Set lossy quality to 80% (0.0 - 100.0)
            
            # The quality / speed trade-off (0=fast, 6=slower-better)
            method: 4         
            
            # With lossless encoding is the ratio of compression to speed.
            # If using lossy encoding this does nothing. 
            # float: 0.0 (worse) - 100.0 inclusive (better but slower).
            # compression: 60             

            threading: true   # Enable multi-threaded encoding.
            
        # The encoding format to serve the image as if not explicitly specified.
        # Defaults to the first enabled encoding format is no set.
        default_serving_format: jpeg
        
        # The default resizing preset to serve images as.
        # If this is not set, the original file sizing is used.
        default_serving_preset: null
        
        presets:
            # Makes a preset named 'small' which can be access when 
            # requesting an image with the `size=small` query parameter.
            small:  
                width: 96    # 96px
                height: 96   # 96px
                
                # The resizing filter to use in order of performance vs quality:
                # 'nearest', 'triangle', 'catmullrom', 
                # 'gaussian' and 'lanczos3' supported.
                filter: triangle    
        
        # The in-memory cache config.
        # If left unset the system will attempt to use the global 
        # cache if enabled, otherwise no caching will be applied.  
        cache:
            # We cache upto the top 100 most recently used images.
            max_images: 100  
            
            # We can also use max_capacity (but not with max_images as well)
            # This will cache by the memory usage limit vs the amount of images.
            # max_capacity: 500  # 500MB limit
        
        # The *bucket local* max upload size allowed for this bucket in KB.
        # No 'realistic' limit is applied if let unset.
        max_upload_size: 2049  # 2MB
        
        # The *bucket local* max concurrent operations.
        # No limit is applied if left unset.
        max_concurrency: 200
```