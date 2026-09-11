pub mod password_input;
pub mod string_input;
pub mod text_based_management_console;

use crate::file_handling::video_processing::local_video_processor::LocalVideoProcessor;

pub trait ManagementConsole {
    fn launch(&'static self, pkg_version: String);
}
