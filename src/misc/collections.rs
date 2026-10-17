use std::{
    any::Any,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub struct DynCloneBox<T: DynClone + ?Sized> {
    value: Box<T>,
}

impl<T: DynClone> DynCloneBox<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Box::new(value),
        }
    }

    // pub fn get(&self) -> &<Self as Deref>::Target {
    //     self.value.deref()
    // }

    pub fn get_box(&self) -> &Box<T> {
        &self.value
    }

    pub fn get_box_mut(&mut self) -> &mut Box<T> {
        &mut self.value
    }
}

impl<T: DynClone> Clone for DynCloneBox<T> {
    fn clone(&self) -> Self {
        let data = self.value.deref();
        Self {
            value: Box::new(DynClone::clone_dyn(data)),
        }
    }
}

impl<T: DynClone> DerefMut for DynCloneBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.deref_mut()
    }
}

impl<T: DynClone> Deref for DynCloneBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.deref()
    }
}

pub trait DynClone {
    fn clone_dyn(self_value: &Self) -> Self
    where
        Self: Sized + DynClone;
}

#[derive(Debug)]
pub struct MutableBox<Z: Clone> {
    value: Option<Z>,
}

impl<Z: Clone> MutableBox<Z> {
    pub const fn new(value: Option<Z>) -> Self {
        Self { value }
    }

    pub fn set(&mut self, value: Option<Z>) {
        self.value = value;
    }

    pub fn get(&self) -> Option<Z> {
        match &self.value {
            Some(val) => Some(val.clone()),
            None => None,
        }
    }
}

impl<Z: Clone> Clone for MutableBox<Z> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

impl<Z: Clone> Deref for MutableBox<Z> {
    type Target = Option<Z>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

// ///Safely gets a VideoProcessor from a VideoProcessorManager (self parameter) by it's id (processor_id parameter). It is to be used like this: get_processor_safe!(self, processor_id);
// macro_rules! get_processor_safe {
//     ($self:expr, $processor_id:expr) => {
//         match $self.get_local_video_processor(&$processor_id) {
//             Some(vp_arc) => Some(vp_arc),
//             None => match $self.get_local_music_processor(&$processor_id) {
//                 Some(vp_arc) => Some(vp_arc),
//                 None => None,
//             },
//         }
//     };
// }

struct ArcBox<T: Any + std::marker::Sized + std::marker::Send + std::marker::Sync + Debug> {
    value: Option<Arc<Mutex<Option<T>>>>,
}

impl<T: Any + std::marker::Sized + std::marker::Send + std::marker::Sync + Debug> ArcBox<T> {
    pub fn set(&mut self, value: Arc<Mutex<Option<T>>>) {
        self.value = Some(value);
    }

    pub fn get(&self) -> Arc<Mutex<Option<T>>> {
        // self.value.clone()
        match &self.value {
            Some(val) => Arc::clone(val),
            None => Arc::new(Mutex::new(None)),
        }
    }

    pub const fn new(value: Arc<Mutex<Option<T>>>) -> Self {
        Self { value: Some(value) }
    }

    pub const fn default_const() -> Self {
        Self { value: None }
    }

    // pub fn initialize(&self, )
}

impl<T: Any + std::marker::Sized + std::marker::Send + std::marker::Sync + Debug> Debug
    for ArcBox<T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArcBox")
            .field("value", &self.value)
            .finish()
    }
}

impl<T: Any + std::marker::Sized + std::marker::Send + std::marker::Sync + Debug> Default
    for ArcBox<T>
{
    fn default() -> Self {
        Self { value: None }
    }
}
