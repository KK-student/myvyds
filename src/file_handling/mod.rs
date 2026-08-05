pub mod file_handling {
    use std::io::ErrorKind;

    use tokio::fs::File;
    use tokio_util::io::ReaderStream;

    ///Get the mime type for a file. Supports only a limited set of mime types.
    pub fn get_mime_type(file_path: &std::path::Path) -> &str {
        match file_path.extension().and_then(std::ffi::OsStr::to_str) {
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("html") => "text/html",
            Some("txt") => "text/plain",
            Some("json") => "application/json",
            Some("js") => "text/javascript",
            Some("jsx") => "text/jsx",
            Some("css") => "text/css",

            //Video and audio:
            Some("webm") => "video/webm",

            //Generic:
            _ => "application/octet-stream", // Default MIME type
        }
    }

    ///Read a file from "./" as a stream.
    pub async fn file_stream_reader(
        file_path_str: &str,
    ) -> Result<ReaderStream<File>, std::io::Error> {
        let path_str = format!("./{}", file_path_str);
        let video_file = match tokio::fs::File::open(&path_str).await {
            Ok(c) => c,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Could not read the file \"{}\"", path_str),
                ));
            }
        };
        let video_stream = ReaderStream::new(video_file);

        return Ok(video_stream);
    }

    pub fn read_small_file(file_path_str: &str) -> Result<Vec<u8>, std::io::Error> {
        let file_path_str = format!("./{}", file_path_str);
        let file_path = std::path::Path::new(&file_path_str);
        if !file_path.exists() {
            let err_str = format!(
                "Error: The file \"{}\" does not exist. ",
                file_path.display()
            );
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, err_str));
        }
        match std::fs::read(file_path) {
            Ok(contents) => Ok(contents),
            Err(_) => Err(std::io::Error::new(
                ErrorKind::Other,
                format!("The file \"{}\" cannot be read. ", file_path.display()),
            )),
        }
    }

    pub fn get_media(file_path_str: &str) -> Result<Vec<u8>, std::io::Error> {
        return read_small_file(format!("./media/{}", file_path_str).as_str());
    }
}
