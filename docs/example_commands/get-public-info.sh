curl -X GET -v "http://127.0.0.1:6000/user/get_public_info?user_id=$1" | jq .
