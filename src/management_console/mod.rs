pub mod string_input;
pub mod text_based_management_console;

pub trait ManagementConsole {
    fn launch(&'static self, pkg_version: String);
}
