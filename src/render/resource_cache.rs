use std::{any::Any, cell::OnceCell};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceId {
    MaterialFlatColor,
}

impl ResourceId {
    const COUNT: usize = 1;
}

pub struct ResourceCache {
    slots: [OnceCell<Box<dyn Any>>; ResourceId::COUNT],
}

impl Default for ResourceCache {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| OnceCell::new()),
        }
    }
}

impl ResourceCache {
    /// # Errors
    /// # Panics
    pub fn get_or_try_init<T: Any>(
        &self,
        id: ResourceId,
        init: impl FnOnce() -> anyhow::Result<T>,
    ) -> anyhow::Result<&T> {
        let cell = &self.slots[id as usize];
        if cell.get().is_none() {
            let _ = cell.set(Box::new(init()?));
        }
        let slot = cell.get().expect("slot was initialized above");
        Ok(slot
            .downcast_ref::<T>()
            .expect("resource slot holds the type registered for its ResourceId"))
    }
}
