curl -X POST -v -F username="$1" session_id="$2" new_text="$3" "http://127.0.0.1:6000/comment/edit?comment_id=$4" | od --endian=big -A d -t u8 -v -
