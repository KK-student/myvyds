#Backend API:

##Getting a video:

1. Client sends a GET request to server with path "/video/get_video_file/[video_id]/output.mpd".

2. If video_id is valid, server sends OK response with the body being the contents of the "output.mpd" file for the video with the cooresponding video_id.

##Getting video basic info:

1. Client sends a GET request to server with path "/video/basic_info?v=[video_id]".

2. If video_id is valid, server sends OK response with the body being the basic details of the video corresponding to the video_id in the following JSON format:

```json
   {
   "name": "[video name]",
   "description": "[video description]",
   "author": "[author's user_id]",
   "uploaded_time": [time uploaded in secs since epoch in u64],
   "likes": [number of likes in u64],
   "dislikes": [number of dislikes in u64]
   }
```

##Getting video comments (page by page):

1. Client sends a GET request to server with path "/video/get_comments_page?v=[video_id],page_index=[page_index]", where page_index is the index of the page in u64, starting from 0.

2. If video_id and page_index is valid, server sends OK response with the body being the list of comments (max 50 comments not including their replies) and their replies, in the following JSON format:

  ```json
  {
    "[comment_id]": {
      "author": "[user_id of comment's author]",
      "text": "[comment text]", 
      "creation_time": [time comment was created in seconds since epoch as a u64],
      "replies": {[replies in the same JSON format as the comment]}
    }
  }
  ```
  Example: 
  ```json
  {
    "23429384": {
      "author": "u1",
      "text": "This is a sample comment. ", 
      "creation_time": 23498237423, 
      "replies": {
        "234432423090": {
          "author": "u2", 
          "text": "This is a sample reply in the same format as the comment u1 made. ", 
          "creation_time": 23499237423,
          "replies": {}
        }
      }
    },
    "more_comments": [True/false: Are there still more comments after this page? ]
  }
  ```

##Getting all video comments: 

1. Client sends a GET request to server with path "/video/get_comments_page?v=[video_id],page_index=-2".

2. If video_id and page_index is valid, server sends OK response with the body being the list of all the video's comments and their replies, in the same format as in step #2 in "Getting video comments (page by page)". 

##Getting a specific comment: 

1. Client sends a GET request to server with path "/comment/get_comment?comment_id=[comment_id]". 

2. If comment_id is valid, server sends OK response with the body being the comment data in the following JSON format: 

```json
{
  "video_id": [video_id of the video that the comment was made for],
  "author": "[author's user_id]",
  "text": "[comment text]", 
  "creation_time": [date and time the comment was created in seconds since epoch as a u64], 
  "replies": {[replies in the same format as the comment, except the replies do not have the "video_id" field]}
}
```

##Getting publicly-available user information: 

1. Client sends a GET request to server with path "/user/get_public_info?user_id=[user_id]". 

2. If user_id is valid, server sends OK response with the body being the publicly available user information in the following JSON format: 

  ```json
  {
    "time_created": [time user was created in secs since epoch in u64]
    "description": "[description]",
    "videos": [video_id, ...],
    "comments": [comment_id, ...],
  }
  ```

##Getting user icon: 

1. Client sends a GET request to server with path "/user/get_user_icon?user_id=[user_id]". 

2. If user_id is valid, server sends OK response with the body being the requested user's icon as a png file. 

##Logging in: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "password"=[hash of the user's password] with path "/user/login". 

2. if username and password is valid, server sends OK response with the body being a randomly-generated session id as a u64 value. Otherwise, server sends an error message with the 401 (unauthorized) error code. 

##Signing up: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "password"=[hash of the user's password] with path "/user/signup".

2. If username is not already taken, server sends OK response with the body being a randomly-generated session id as a u64 value. Otherwise, server sends an error message. TODO: Define specific error code. 
