//! Unit model history has independent entry and heap-capacity limits.
use super::{HashMap, VecDeque};
use crate::voxels::VoxelModel;

pub(super) struct ModelCache {
    entries: HashMap<String, Option<VoxelModel>>,
    order: VecDeque<String>,
    bytes: usize,
    max_entries: usize,
    max_bytes: usize,
}
impl Default for ModelCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            max_entries: 128,
            max_bytes: 128 * 1024 * 1024,
        }
    }
}
impl ModelCache {
    pub(super) fn contains_key(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }
    pub(super) fn get(&self, id: &str) -> Option<&Option<VoxelModel>> {
        self.entries.get(id)
    }
    pub(super) fn insert(&mut self, id: String, model: Option<VoxelModel>) {
        let bytes = model.as_ref().map_or(0, VoxelModel::retained_bytes);
        if self.contains_key(&id) || self.max_entries == 0 || bytes > self.max_bytes {
            return;
        }
        while self.entries.len() >= self.max_entries
            || bytes > self.max_bytes.saturating_sub(self.bytes)
        {
            let Some(old) = self.order.pop_front() else {
                return;
            };
            if let Some(model) = self.entries.remove(&old) {
                self.bytes -= model.as_ref().map_or(0, VoxelModel::retained_bytes);
            }
        }
        self.bytes += bytes;
        self.order.push_back(id.clone());
        self.entries.insert(id, model);
    }
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
    #[cfg(test)]
    pub(super) fn limits(&mut self, entries: usize, bytes: usize) {
        assert!(self.entries.is_empty());
        self.max_entries = entries;
        self.max_bytes = bytes;
    }
    #[cfg(test)]
    pub(super) fn stats(&self) -> (usize, usize, usize) {
        (self.entries.len(), self.bytes, self.order.len())
    }
}
