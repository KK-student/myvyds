curl -X POST -v -F username="$1" -F password="$2" "http://localhost:6000/user/login" --output - | od --endian=big -A d -t u8 -v -
