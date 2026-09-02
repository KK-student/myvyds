# TODO: Need to figure out how to include the artwork as well. This command only includes the audio. 
ffmpeg -i "$1" -c:v copy -map 0:a:0 -c:a:0 flac -b:a:0 192k -map 0:a:0 -c:a:1 flac -b:a:1 128k -map 0:a:0 -c:a:2 flac -b:a:2  64k -map_metadata 0 -f dash ./output.mpd
