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

```