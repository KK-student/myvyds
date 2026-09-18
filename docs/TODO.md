# Things I still need to implement (x means done)

# Backend
## [ ] Videos: 
- [x] Getting a video.
- [x] Getting video basic info.
- [x] Getting video comments (page by page).
- [x] Getting all video comments.
- [x] Getting video thumbnail image.
- [ ] Uploading a video.
  * [x] Implemented LocalVideoProcessor. 
  * [x] Implemented LocalMusicProcessor. 
  * [x] Implemented VideoProcessorManager. 
  * [x] Implement a console that would allow the admin to view and manage all of the video processors. 
  * [ ] Implemented LocalArchiveVideoProcessor. 
  * [ ] Implemented LocalArchiveMusicProcessor. 
  * [ ] Implemented SSHVideoProcessor. 
  * [ ] Implemented SSHMusicProcessor. 
  * [ ] Implemented the upload video endpoint on the server. 
  * [ ] Implemented save_data function for VideoProcessorManager. 
  * [ ] (Not optional) Add the ability for VideoProcessorManager to add videos that can't be processed right now to a queue and then processed later when possible. 
- [x] Checking if a video is still being processed.
- [ ] Editing a video.
- [ ] Removing a video.
- [ ] Implemented live streaming using the ez-ffmpeg crate. 

## [x] Users: 
- [x] Getting publicly-available user information.
- [x] Getting user icon.
- [x] Logging in.
- [x] Signing up.
- [x] Add the ability to create, delete, and edit user data in the TextBasedManagementConsole.
- [x] Getting user's subscriptions.
- [x] Getting user's liked videos.
- [x] Getting user's disliked videos.
- [x] Getting user's personal comments and replies.
- [x] Getting user's liked comments and replies.
- [x] Getting user's disliked comments and replies.

## [ ] Authentication: 
- [x] Refactor authentication code. 

## [ ] Comments: 
- [x] Getting a specific comment or reply.
- [x] Making a comment.
- [x] Editing a comment or reply.
- [ ] Adding a reply to a comment.
- [ ] Removing a comment or reply.

# Frontend (Still need to create a TODO list for this)
