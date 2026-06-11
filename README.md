## dumahhfiles

yt-dlp client

sister project of [bumahhfiles](https://github.com/prashantrahul141/bumahhfiles)

### Configuration

Configuration can be done via environment variables
| name                       | default           | purpose                                      |
|----------------------------|-------------------|----------------------------------------------|
| DUMAHH_ROOT_DIR            | "./files"         | where to store files                         |
| DUMAHH_INTERNAL_HOST       | "0.0.0.0"         | where to listen                              |
| DUMAHH_INTERNAL_PORT       | 3000              | which port to listen                         |
| DUMAHH_EXTERNAL_PROTOCOL   | "http"            | used to format links                         |
| DUMAHH_EXTERNAL_HOST       | "0.0.0.0:3000"    | used to format links                         |
| DUMAHH_MAX_FILENAME_LENGTH | 240               | max filename in storage                      |
| DUMAHH_MAX_ON_DISK_STORAGE | 5368709120 (5GB)  | max storage allowed                          |
| DUMAHH_MAX_FILE_SIZE       | 104857600 (100MB) | max per file size                            |
| DUMAHH_RETENTION_MINS      | 3 (mins)          | how long to keep files                       |
| DUMAHH_CONCURRENT_DOWNLOAD | 3                 | concurrent download limit                    |
| DUMAHH_REQUESTS_PER_MINUTE | 30 (per minute)   | rate limiter                                 |
| DUMAHH_PASSWORD            | not set           | password protect, dont set to allow anyone   |
| DUMAHH_YTDLP_PATH          | "yt-dlp"          | path to yt-dlp binary                        |
| DUMAHH_COOKIES_FILEPATH    | not set           | path to cookies file given to ytdlp          |
| RUST_LOG                   | "debug"           | logging level                                |
| version                    | "unknown"         | current commit hash                          |


### Retention

Files are automatically deleted after {DUMAHH_RETENTION_MINS} minutes.
