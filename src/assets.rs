//! Asset side of the scene graph: asset keys and the repositories owning the
//! actual resources.
//!
//! [`Scene`](crate::scene::Scene) nodes reference assets only by key
//! ([`MeshKey`] / [`MaterialKey`]); the repositories own the real resources.
//! Keys are declared here rather than in `scene.rs` because they are bound to
//! the stores that interpret them, not to the graph that references them.

use slotmap::{Key, SlotMap, new_key_type};

use crate::{
    gpu::allocator::MemAllocator,
    material::Material,
    mesh::{Mesh, MeshData},
};

new_key_type! {
    /// Key into a [`MeshRepository`]; stored by a scene node's `mesh` field.
    pub struct MeshKey;

    /// Key into a [`MaterialRepository`]; stored by a scene node's `material` field.
    pub struct MaterialKey;
}

pub struct Repository<K: Key, T> {
    slots: SlotMap<K, T>,
}

impl<K: Key, T> Default for Repository<K, T> {
    fn default() -> Self {
        Self {
            slots: SlotMap::with_key(),
        }
    }
}

impl<K: Key, T> Repository<K, T> {
    /// Store `resource` and return its key.
    fn insert(&mut self, resource: T) -> K {
        self.slots.insert(resource)
    }

    /// Fetch a reference by key.
    #[must_use]
    fn get(&self, key: K) -> Option<&T> {
        self.slots.get(key)
    }

    /// Remove and return a resource by key; afterwards the key is invalid.
    fn remove(&mut self, key: K) -> Option<T> {
        self.slots.remove(key)
    }

    /// Whether `key` can be resolved into a stored resource.
    #[must_use]
    fn contains(&self, key: K) -> bool {
        self.slots.contains_key(key)
    }

    /// Iterate `(key, resource)` pairs in storage order.
    fn iter(&self) -> impl Iterator<Item = (K, &T)> + '_ {
        self.slots.iter()
    }

    /// Number of stored resources.
    #[must_use]
    fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether no resource is stored.
    #[must_use]
    fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

pub type MaterialRepository = Repository<MaterialKey, Material>;

pub struct MeshRepository<'a> {
    allocator: &'a MemAllocator<'a>,
    repo: Repository<MeshKey, Mesh>,
}

impl<'a> MeshRepository<'a> {
    #[must_use]
    pub fn new(allocator: &'a MemAllocator) -> Self {
        Self {
            allocator,
            repo: Repository::default(),
        }
    }

    /// # Errors
    pub fn add_mesh_from_mesh_data(&mut self, mesh_data: &MeshData) -> anyhow::Result<MeshKey> {
        let mesh = self.allocator.allocate_mesh(mesh_data)?;
        Ok(self.repo.insert(mesh))
    }

    #[must_use]
    pub fn get(&self, key: MeshKey) -> Option<&Mesh> {
        self.repo.get(key)
    }

    pub fn remove(&mut self, key: MeshKey) -> Option<Mesh> {
        self.repo.remove(key)
    }
}
