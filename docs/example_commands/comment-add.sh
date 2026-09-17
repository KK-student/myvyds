curl -X POST -v -F username="$1" -F session_id="$2" -F text="$3" "http://127.0.0.1:6000/comment/add" | od --endian=big -A d -t u8 -v -
