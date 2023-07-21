# cda_copy
Allows you to automatically copy, combine and convert all files on an audio CD into a convenient mp3.

## Usage:
`cda_copy <OUTPUT_FILE_NAME> [OPTIONS]`
### Options:
`-b/--bitrate`: Allows you to set the bitrate for ffmpeg encoding. The default value is 192k, resulting in about 100MB per CD.
`-s/--skip-tagging`: Skip acquiring/adding mp3 tags
`-n/--num-disks`: Set the number of disks to be combined (used for audio books etc. that are spread across multiple disks)

## Examples:
`cda_copy foo.mp3 -bitrate 256k`

## Requirements:
ffmpeg
gio

