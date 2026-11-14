//! Bounded decoded sprite history; protected request layers stay together.

use super::{HashMap, Sprite, VecDeque};
use std::hash::Hash;

const MAX_ENTRIES: usize = 512;
const MAX_BYTES: usize = 128 * 1024 * 1024;

pub(super) struct DecodedCache<K> {
    entries: HashMap<K, Option<Sprite>>,
    order: VecDeque<K>,
    bytes: usize,
    max_entries: usize,
    max_bytes: usize,
}

impl<K> Default for DecodedCache<K> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            max_entries: MAX_ENTRIES,
            max_bytes: MAX_BYTES,
        }
    }
}

impl<K: Clone + Eq + Hash> DecodedCache<K> {
    pub(super) fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    pub(super) fn get(&self, key: &K) -> Option<&Option<Sprite>> {
        self.entries.get(key)
    }

    /// Call only for a missing key. Failed decodes consume an entry too.
    /// If the current building exceeds the budget, omit its later optional
    /// layers and retry them on a future request rather than caching failure.
    pub(super) fn insert(&mut self, key: K, sprite: Option<Sprite>, protected: &[&K]) {
        let bytes = sprite.as_ref().map_or(0, |s| s.rgba.capacity());
        if self.max_entries == 0 || bytes > self.max_bytes {
            return;
        }
        while self.entries.len() >= self.max_entries
            || bytes > self.max_bytes.saturating_sub(self.bytes)
        {
            let Some(index) = self.order.iter().position(|old| !protected.contains(&old)) else {
                return;
            };
            if let Some(old) = self.order.remove(index) {
                if let Some(Some(sprite)) = self.entries.remove(&old) {
                    self.bytes -= sprite.rgba.capacity();
                }
            }
        }
        self.bytes += bytes;
        self.order.push_back(key.clone());
        self.entries.insert(key, sprite);
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub(super) fn with_limits(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            max_entries,
            max_bytes,
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub(super) fn stats(&self) -> (usize, usize, usize) {
        (self.entries.len(), self.bytes, self.order.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprites::LayerKey;

    fn key(frame: usize) -> LayerKey {
        LayerKey {
            file: "WORK.SHP".into(),
            frame,
            shadow: true,
            remap: [0; 3],
            offset: (0, 0),
        }
    }

    fn sprite(bytes: usize) -> Option<Sprite> {
        Some(Sprite {
            w: 1,
            h: 1,
            ax: 0,
            ay: 0,
            rgba: vec![0; bytes],
        })
    }

    #[test]
    fn byte_pressure_evicts_oldest_unprotected_and_counts_allocated_capacity() {
        let mut cache = DecodedCache::with_limits(8, 12);
        for i in 0..3 {
            cache.insert(key(i), sprite(4), &[]);
        }
        cache.insert(key(3), sprite(8), &[&key(0)]);
        assert_eq!((cache.len(), cache.bytes, cache.order.len()), (2, 12, 2));
        assert!(cache.get(&key(0)).unwrap().is_some());
        assert!(cache.get(&key(3)).unwrap().is_some());
        assert!(cache.get(&key(1)).is_none());
        assert!(cache.get(&key(2)).is_none());

        let mut reserved = sprite(8).unwrap();
        reserved.rgba.truncate(4);
        cache.insert(key(4), Some(reserved), &[]);
        assert_eq!((cache.len(), cache.bytes, cache.order.len()), (1, 8, 1));
        assert_eq!(cache.get(&key(4)).unwrap().as_ref().unwrap().rgba.len(), 4);
    }

    #[test]
    fn missing_art_is_bounded_and_protected_working_sets_do_not_cache_budget_failure() {
        let mut cache = DecodedCache::with_limits(2, 8);
        for i in 0..600 {
            cache.insert(key(i), None, &[]);
            assert!(cache.len() <= 2);
            assert_eq!(cache.bytes, 0);
            assert_eq!(cache.order.len(), cache.len());
        }
        cache.insert(key(0), sprite(4), &[]);
        cache.insert(key(1), sprite(4), &[&key(0)]);
        cache.insert(key(2), sprite(4), &[&key(0), &key(1)]);
        assert!(
            cache.get(&key(2)).is_none(),
            "budget failure remains retryable"
        );
        assert_eq!((cache.len(), cache.bytes), (2, 8));
        cache.insert(key(2), sprite(4), &[&key(0)]);
        assert!(cache.get(&key(2)).unwrap().is_some());
        assert!(cache.get(&key(0)).unwrap().is_some());
        assert!(cache.get(&key(1)).is_none());

        cache.insert(key(3), sprite(9), &[]);
        assert_eq!(
            (cache.len(), cache.bytes),
            (2, 8),
            "impossible entry keeps neighbors"
        );
        assert!(cache.get(&key(3)).is_none());
    }
}
