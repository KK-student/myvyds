curl -X GET -v "http://127.0.0.1:6000/comment/get_comment?comment_id=$1" | jq .
