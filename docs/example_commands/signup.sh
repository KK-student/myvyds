curl -X POST -v -F "username"="$1" -F "password"="$2" "http://127.0.0.1:6000/user/signup" | od --endian=big -A d -t u8 -v -
