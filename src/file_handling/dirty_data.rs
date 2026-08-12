pub trait DirtyData {
    ///Sets the dirty flag. True means that the data has been modified and needs to be saved somewhere.
    fn set_dirty(&mut self, dirty: bool);

    ///Gets the current value of the dirty flag. True means that the data has been modified and needs to be saved somewhere.

    fn is_dirty(&self) -> bool;
}
