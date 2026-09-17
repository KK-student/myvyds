curl -X POST -v -F username="$1" -F session_id="$2" "http://127.0.0.1:6000/user/get_liked_comments" | jq .
