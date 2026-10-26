//! The scene graph: nodes in a parent/child tree, each with a local
//! [`Transform`] and visibility flag, flattened after world-space update
//! into [`DrawCommand`]s for the renderer.

use glam::Mat4;
use slotmap::{SlotMap, new_key_type};
use thiserror::Error;

use crate::assets::{MaterialKey, MeshKey};
use crate::light::MainLight;
use crate::math::Transform;

new_key_type! {
    pub struct NodeKey;
}

pub struct Node {
    pub name: String,
    pub transform: Transform,
    pub world_matrix: Mat4,
    pub parent: Option<NodeKey>,
    pub children: Vec<NodeKey>,
    pub visible: bool,
    pub mesh: Option<MeshKey>,
    pub material: Option<MaterialKey>,
}

impl Node {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transform: Transform::default(),
            world_matrix: Mat4::IDENTITY,
            parent: None,
            children: Vec::new(),
            visible: true,
            mesh: None,
            material: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DrawCommand {
    pub node_key: NodeKey,
    pub mesh_key: MeshKey,
    pub material_key: MaterialKey,
    pub model_matrix: Mat4,
}

#[derive(Error, Debug)]
pub enum SceneError {
    #[error("cannot parent a node to itself")]
    ParentEqualChild,
    #[error("node not exist in Scene")]
    NodeNotFound,
}

pub struct Scene {
    nodes: SlotMap<NodeKey, Node>,
    main_light: MainLight,
    clear_color: [f32; 4],
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            main_light: MainLight::None,
            clear_color: [0.0; 4],
        }
    }
}

impl Scene {
    pub const fn set_main_light(&mut self, light: MainLight) {
        self.main_light = light;
    }

    pub const fn set_clear_color(&mut self, color: [f32; 4]) {
        self.clear_color = color;
    }

    #[must_use]
    pub const fn main_light(&self) -> MainLight {
        self.main_light
    }

    #[must_use]
    pub const fn clear_color(&self) -> [f32; 4] {
        self.clear_color
    }

    pub fn spawn_node(&mut self, mut node: Node) -> NodeKey {
        node.parent = None;
        node.children.clear();
        self.nodes.insert(node)
    }

    #[must_use]
    pub fn get(&self, key: NodeKey) -> Option<&Node> {
        self.nodes.get(key)
    }

    pub fn get_mut(&mut self, key: NodeKey) -> Option<&mut Node> {
        self.nodes.get_mut(key)
    }

    /// # Errors
    ///
    /// * [`SceneError::ParentEqualChild`] if a node is parented to itself
    /// * [`SceneError::NodeNotFound`] if `parent` or `child` is unknown
    pub fn attach(&mut self, parent: NodeKey, child: NodeKey) -> Result<(), SceneError> {
        if parent == child {
            return Err(SceneError::ParentEqualChild);
        }
        if !self.nodes.contains_key(parent) || !self.nodes.contains_key(child) {
            return Err(SceneError::NodeNotFound);
        }

        // Detach from previous parent if one exists
        self.detach(child);

        self.nodes[child].parent = Some(parent);
        self.nodes[parent].children.push(child);

        Ok(())
    }

    pub fn detach(&mut self, child: NodeKey) {
        let old_parent = self
            .nodes
            .get_mut(child)
            .and_then(|node| node.parent.take());
        if let Some(old_parent) = old_parent
            && let Some(parent_node) = self.nodes.get_mut(old_parent)
        {
            parent_node.children.retain(|&c| c != child);
        }
    }

    pub fn remove_node(&mut self, key: NodeKey) {
        if !self.nodes.contains_key(key) {
            return;
        }

        // Unlink from parent first
        self.detach(key);

        // Remove the whole subtree, traversing with a stack
        let mut stack = vec![key];
        while let Some(current_key) = stack.pop() {
            if let Some(node) = self.nodes.remove(current_key) {
                stack.extend(node.children);
            }
        }
    }

    pub fn update_world_matrix(&mut self) {
        let roots: Vec<NodeKey> = self
            .nodes
            .iter()
            .filter_map(|(key, node)| {
                if node.parent.is_none() {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();

        for root in roots {
            Self::propagate_transforms(&mut self.nodes, root, Mat4::IDENTITY);
        }
    }

    fn propagate_transforms(nodes: &mut SlotMap<NodeKey, Node>, key: NodeKey, parent_world: Mat4) {
        let (local_matrix, children) = match nodes.get(key) {
            Some(node) => (node.transform.to_matrix(), node.children.clone()),
            None => return,
        };

        let world_matrix = parent_world * local_matrix;

        if let Some(node) = nodes.get_mut(key) {
            node.world_matrix = world_matrix;
        }

        for child_key in children {
            Self::propagate_transforms(nodes, child_key, world_matrix);
        }
    }

    /// Extract a [`DrawCommand`] list for every visible, drawable node.
    ///
    /// Traverses the scene graph depth-first from the roots in pre-order, so
    /// an invisible node hides its entire subtree, not just itself. Assumes [`Self::update_world_matrix`] ran first to keep the
    /// `world_matrix`s fresh.
    #[must_use]
    pub fn extract_draw_list(&self) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        for (key, node) in &self.nodes {
            if node.parent.is_none() {
                Self::collect_draws(&self.nodes, key, &mut commands);
            }
        }
        commands
    }

    /// Pre-order DFS that emits only visible, drawable nodes; an invisible
    /// node culls its whole subtree.
    fn collect_draws(
        nodes: &SlotMap<NodeKey, Node>,
        key: NodeKey,
        commands: &mut Vec<DrawCommand>,
    ) {
        let Some(node) = nodes.get(key) else {
            return;
        };
        if !node.visible {
            return;
        }

        if let (Some(mesh_key), Some(material_key)) = (node.mesh, node.material) {
            commands.push(DrawCommand {
                node_key: key,
                mesh_key,
                material_key,
                model_matrix: node.world_matrix,
            });
        }

        for &child in &node.children {
            Self::collect_draws(nodes, child, commands);
        }
    }
}
