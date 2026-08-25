use std::sync::Mutex;

use crate::file_handling::{file_handling::DataManager, video_processing::VideoProcessorManager};

pub struct ManagementConsole {
    data_manager: &'static std::sync::LazyLock<Mutex<DataManager>>,
    processor_manager: &'static VideoProcessorManager,
}

impl ManagementConsole {
    pub const fn new(
        data_manager: &'static std::sync::LazyLock<Mutex<DataManager>>,
        processor_manager: &'static VideoProcessorManager,
    ) -> Self {
        return Self {
            data_manager,
            processor_manager,
        };
    }
}
