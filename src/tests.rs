#[cfg(test)]
pub mod test_functions {
    use std::{
        ops::{Deref, DerefMut},
        path::Path,
        sync::{LazyLock, Mutex},
    };

    use crate::{
        DATA_MANAGER, PROCESSOR_MANAGER, file_handling::video_processing::video_processor::BoolBox,
    };

    use super::*;

    static INIT_STATE: LazyLock<Mutex<BoolBox>> =
        LazyLock::new(|| -> Mutex<BoolBox> { Mutex::new(BoolBox::new(false)) });

    fn initialize_data() {
        if is_data_initialized() {
            return;
        }

        println!(
            "Current directory: \"{:?}\"",
            std::fs::canonicalize(".").unwrap().to_str().unwrap()
        );

        let data_dir: String = "./data.empty-sample".to_string();

        let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager = data_manager_mutg.deref_mut();
        data_manager.set_data_path(&data_dir);
        data_manager.load_data();

        PROCESSOR_MANAGER.set_data_dir(Path::new(data_dir.as_str()));
        PROCESSOR_MANAGER.load_data();

        let mut i_mutg = INIT_STATE.lock().unwrap();
        let i = i_mutg.deref_mut();
        i.set(true);
    }

    fn is_data_initialized() -> bool {
        let i_mutg = INIT_STATE.lock().unwrap();
        i_mutg.deref().get()
    }

    #[test]
    pub fn test_loading() {
        initialize_data();
    }
}
