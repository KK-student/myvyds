# Backend API:

## Videos: 
### Getting a video:

1. Client sends a GET request to server with path "/video/get_video_file/[video_id]/output.mpd".

2. If video_id is valid, server sends OK response with the body being the contents of the "output.mpd" file for the video with the cooresponding video_id. Otherwise, server sends error response with error code 404. 

3. Client then sends GET requests to server with path "/video/get_video_file/[video_id]/[MPEG_DASH segment file]" to get each MPEG_DASH segment file to play the video. If video_id is valid and the requested file exists, then server sends OK response with the body being the contents of the requested file. Otherwise, server sends error response with error code 404. 

### Getting video basic info:

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

Otherwise, server sends error response with error code 404. 

### Getting video comments (page by page):

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
  }
  ```

Otherwise, server sends error response with error code 404. 

### Getting all video comments: 

1. Client sends a GET request to server with path "/video/get_comments_page?v=[video_id],page_index=-2".

2. If video_id and page_index is valid, server sends OK response with the body being the list of all the video's comments and their replies, in the same format as in step #2 in "Getting video comments (page by page)". Otherwise, server sends error response with error code 404. 

### Getting video thumbnail image: 

1. Client sends a GET request to server with path "/video/thumbnail_image?v=[video_id]". 

2. If video_id is valid, server sends OK response with the body being the thumbnail image itself. Otherwise, server sends error response with error code 404. 

### Uploading a video: 

1. Client sends a form to the server with the fields "username"=[user's user_id], "session_id"=[user's current session_id], "title"=[title of the video], "description"=[description of the video], "video_file"=[the video file as multipart data in supported file format with supported codecs], "thumbnail_video"=[the thumbnail video file as multipart data in supported file format with supported codecs], and "thumbnail_image"=[the thumbnail image file as multipart data in png format] to the endpoint "/video/upload". 

2. 
- If username and session_id are valid: 
  - If video title is not already being used for another video: 
    - If video file format and audio/video codecs are supported: 
      - If server determines that it has enough storage: Server sends OK response with status code 201 (Created) with the body being the video_id of the newly-uploaded video, which, in a separate thread, the server will be processing to create the needed MPEG_DASH files. 
      - Otherwise, server sends error message with the error code 507 (Insufficient Storage). 
    - Otherwise, server sends error message with the error code 415 (Unsupported Media Type). 
  - Otherwise, server sends error message with the error code 409 (Conflict). 
- Otherwise, server sends error message with the error code 401 (Unauthorized). 

### Checking if a video is still being processed: 

1. Client sends a GET request to the server to the endpoint "/video/is_currently_processing?v=[video_id]" with the body of the request being a form with the fields "username"=[user's user_id] and "session_id"=[user's current session_id]. 

2. 
- If username and session_id are valid: 
  - If the video with the given video_id exists: Server sends either "{"video_id":[video_id], processing_status": 1}" if the video is still being processed, "{"video_id":[video_id], processing_status": 0}" if successfully finished processing, "{"video_id":[video_id], processing_status": 4}" if it hasn't even started processing, or "{"video_id":[video_id], processing_status": 2, "error_message": "[error_message]"}" if it failed to process. 
  - Otherwise, server sends error message with the error code 404. 
- Otherwise, server sends error message with the error code 401 (Unauthorized). 

### Editing a video: 

Still needs to be implemented in DataManager and VideoData structs. 

### Removing a video: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "session_id"=[user's current session_id] to the endpoint "/video/remove?v=[video_id]". 

2. 
- If username and session_id are valid (including user owns video): Server deletes the video with all of it's comments and replies and replies with an OK response. 
- Otherwise, server sends error message with the error code 401 (Unauthorized). 

## Users: 
### Getting publicly-available user information: 

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
Otherwise, server sends error response with the 404 error code. 

### Getting user icon: 

1. Client sends a GET request to server with path "/user/get_user_icon?user_id=[user_id]". 

2. If user_id is valid, server sends OK response with the body being the requested user's icon as a png file. Otherwise, server sends error response with error code 404. 


### Logging in: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "password"=[the user's password] with path "/user/login". 

2. if username and password is valid, server sends OK response with the body being a randomly-generated session_id as a u64 value which will only be valid for 2h (after which the user will have to log in again). Otherwise, server sends an error message with the 401 (unauthorized) error code. 

### Signing up: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "password"=[the user's password] with path "/user/signup".

2. If username is not already taken, server sends OK response with the body being a randomly-generated session_id as a u64 value which will only be valid for 2h (after which the user will have to log in to get a new session_id). Otherwise, server sends an error message with the 400 (Bad Request) error code. 

### Getting user's subscriptions: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "session_id"=[user's session_id] to the endpoint "/user/get_subscriptions".

2. 
- If username and session_id are valid: Server replies with an OK response with the body being the list of subscriptions in the following JSON format: 

```json
{
  "subscriptions": [list of user's subscriptions as an array of strings]
}
```

- Otherwise, server sends error message with the error code 401 (Unauthorized). 

### Getting user's liked videos: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "session_id"=[user's session_id] to the endpoint "/user/get_liked_videos".

2. 
- If username and session_id are valid: Server replies with an OK response with the body being the list of user's liked videos in the following JSON format: 

```json
{
  "liked_videos": [list of user's liked videos as an array of u64s]
}
```

- Otherwise, server sends error message with the error code 401 (Unauthorized). 

### Getting user's disliked videos: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "session_id"=[user's session_id] to the endpoint "/user/get_disliked_videos".

2. 
- If username and session_id are valid: Server replies with an OK response with the body being the list of user's disliked videos in the following JSON format: 

```json
{
  "disliked_videos": [list of user's disliked videos as an array of u64s]
}
```

- Otherwise, server sends error message with the error code 401 (Unauthorized). 

### Getting user's comments and replies: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "session_id"=[user's session_id] to the endpoint "/user/get_user_comments".

2. 
- If username and session_id are valid: Server replies with an OK response with the body being the list of user's liked comments in the following JSON format: 

```json
{
  "user_comments": [list of user's comments and replies as an array of comment_ids]
}
```

- Otherwise, server sends error message with the error code 401 (Unauthorized). 

### Getting user's liked comments and replies: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "session_id"=[user's session_id] to the endpoint "/user/get_liked_comments".

2. 
- If username and session_id are valid: Server replies with an OK response with the body being the list of user's liked comments in the following JSON format: 

```json
{
  "liked_comments": [list of user's liked comments as an array of comment_ids]
}
```

- Otherwise, server sends error message with the error code 401 (Unauthorized). 

### Getting user's disliked comments and replies: 

1. Client sends a form to the server with the fields "username"=[user's user_id] and "session_id"=[user's session_id] to the endpoint "/user/get_disliked_comments".

2. 
- If username and session_id are valid: Server replies with an OK response with the body being the list of user's disliked comments in the following JSON format: 

```json
{
  "disliked_comments": [list of user's disliked comments as an array of comment_ids]
}
```

- Otherwise, server sends error message with the error code 401 (Unauthorized). 

## Comments: 
### Getting a specific comment or reply: 

1. Client sends a GET request to server with path "/comment/get_comment?comment_id=[comment_id]". 

2. If comment_id is valid, server sends OK response with the body being the comment data in the following JSON format: 

```json
{
  "video_id": [video_id of the video that the comment was made for],
  "can_have_replies": [whether or not replies can be made to this comment/reply as a true/false boolean value]
  "author": "[author's user_id]",
  "text": "[comment text]", 
  "creation_time": [date and time the comment was created in seconds since epoch as a u64], 
  "replies": {[replies in the same format as the comment, except the replies do not have the "video_id" field]}
}
```

### Making a comment: 

1. Client sends a GET request to the server to the endpoint "/comment/add?v=[video_id]" with the body of the request being a form with the fields "username"=[user's user_id], "session_id"=[user's current session_id], and "text"=[comment text]. 

2. 
- If username and session_id are valid: 
  - If video_id is valid: Server creates the comment and responds with an OK message with the body being the comment_id as a u64 value. 
  - Otherwise, server sends an error message with the error code 404. 
- Otherwise, server sends error message with the error code 401 (Unauthorized). 

### Editing a comment or reply: 

1. Client sends a GET request to the server to the endpoint "/comment/edit?comment_id=[comment_id]" with the body of the request being a form with the fields "username"=[user's user_id], "session_id"=[user's current session_id], and "text"=[comment text]. 

2. 
- If username and session_id are valid: 
  - If comment_id is valid: Server edits the comment and responds with an OK message with the body being empty. 
  - Otherwise, server sends an error message with the error code 404. 
- Otherwise, server sends error message with the error code 401 (Unauthorized). 

### Adding a reply to a comment: 

1. Client sends a GET request to the server to the endpoint "/comment/add_reply?comment_id=[comment_id]" with the body of the request being a form with the fields "username"=[user's user_id], "session_id"=[user's current session_id], and "text"=[comment text]. 

2. 
- If username and session_id are valid: 
  - If comment_id is valid: 
    - If replies could be added to the comment: Server adds a reply to the comment and responds with an OK message with the body being the comment_id of the reply as a u64 value. 
    - Otherwise, server responds with an error message with the error code 409 (Conflict). 
  - Otherwise, server sends an error message with the error code 404. 
- Otherwise, server sends error message with the error code 401 (Unauthorized). 

### Removing a comment or reply: 

1. Client sends a GET request to the server to the endpoint "/comment/remove?comment_id=[comment_id]" with the body of the request being a form with the fields "username"=[user's user_id], "session_id"=[user's current session_id]. 

2. 
- If username and session_id are valid: 
  - If comment_id is valid: Server deletes the comment and responds with an OK message with the body being empty. 
  - Otherwise, server sends an error message with the error code 404. 
- Otherwise, server sends error message with the error code 401 (Unauthorized). 
