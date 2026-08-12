pub trait DirtyData {
    fn set_dirty(&mut self, dirty: bool);
    fn is_dirty(&self) -> bool;
}
