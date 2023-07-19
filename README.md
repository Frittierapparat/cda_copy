# cda_copy
Allows you to automatically copy, combine and convert all files on an audio CD into a convenient mp3.

## Usage:
`cda_copy <OUTPUT_FILE_NAME> [OPTIONS]`
### Options:
`-b/--bitrate`: Allows you to set the bitrate for ffmpeg encoding. The default value is 192k, resulting in about 100MB per CD.
`-s/--skip-tagging`: Skip acquiring/adding mp3 tags

## Examples:
`cda_copy foo.mp3 -bitrate 256k`

## Requirements:
ffmpeg
sox
gio

