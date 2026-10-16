//! Asset side of the scene graph: asset keys and the repositories owning the
//! actual resources.
//!
//! [`Scene`](crate::scene::Scene) nodes reference assets only by key
//! ([`MeshKey`] / [`MaterialKey`]); the repositories own the real resources.
//! Keys are declared here rather than in `scene.rs` because they are bound to
//! the stores that interpret them, not to the graph that references them.

use slotmap::{Key, SlotMap, new_key_type};

new_key_type! {
    /// Key into a [`MeshRepository`]; stored by a scene node's `mesh` field.
    pub struct MeshKey;

    /// Key into a [`MaterialRepository`]; stored by a scene node's `material` field.
    pub struct MaterialKey;
}

/// `slotmap`-backed store mapping keys of type `K` to values of type `T`.
///
/// Co-existing generic instantiations over distinct `slotmap` key types
/// ([`MeshRepository`] and [`MaterialRepository`]) are never interchangeable.
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
    pub fn insert(&mut self, resource: T) -> K {
        self.slots.insert(resource)
    }

    /// Fetch a reference by key.
    #[must_use]
    pub fn get(&self, key: K) -> Option<&T> {
        self.slots.get(key)
    }

    /// Fetch a mutable reference by key.
    pub fn get_mut(&mut self, key: K) -> Option<&mut T> {
        self.slots.get_mut(key)
    }

    /// Remove and return a resource by key; afterwards the key is invalid.
    pub fn remove(&mut self, key: K) -> Option<T> {
        self.slots.remove(key)
    }

    /// Whether `key` can be resolved into a stored resource.
    #[must_use]
    pub fn contains(&self, key: K) -> bool {
        self.slots.contains_key(key)
    }

    /// Iterate `(key, resource)` pairs in storage order.
    pub fn iter(&self) -> impl Iterator<Item = (K, &T)> + '_ {
        self.slots.iter()
    }

    /// Number of stored resources.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether no resource is stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// Stores meshes; scene nodes reference them by [`MeshKey`] (e.g. [`Node::mesh`](crate::scene::Node)).
///
/// The payload is generic until the concrete `Mesh` type is in place.
pub type MeshRepository<T> = Repository<MeshKey, T>;

/// Stores materials; scene nodes reference them by [`MaterialKey`] (e.g. [`Node::material`](crate::scene::Node)).
///
/// The payload is generic until the concrete `Material` type is in place.
pub type MaterialRepository<T> = Repository<MaterialKey, T>;
