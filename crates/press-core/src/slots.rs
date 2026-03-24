// SPDX-License-Identifier: GPL-3.0-or-later
//! Generational storage. A handle packs a slot index and the generation the
//! slot had when the handle was issued, so a handle to a destroyed object is
//! detected instead of silently reaching whatever reused the slot.

/// An opaque object reference: `(generation << 32) | (index + 1)`. Zero is null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(pub u64);

impl Handle {
    pub const NULL: Handle = Handle(0);

    fn new(index: usize, generation: u32) -> Handle {
        Handle(((generation as u64) << 32) | (index as u64 + 1))
    }

    fn parts(self) -> Option<(usize, u32)> {
        let low = self.0 & 0xffff_ffff;
        if low == 0 {
            None
        } else {
            Some((low as usize - 1, (self.0 >> 32) as u32))
        }
    }
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

pub struct Slots<T> {
    slots: Vec<Slot<T>>,
    free: Vec<usize>,
}

impl<T> Default for Slots<T> {
    fn default() -> Self {
        Slots { slots: Vec::new(), free: Vec::new() }
    }
}

impl<T> Slots<T> {
    pub fn insert(&mut self, value: T) -> Handle {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index];
            slot.value = Some(value);
            Handle::new(index, slot.generation)
        } else {
            self.slots.push(Slot { generation: 1, value: Some(value) });
            Handle::new(self.slots.len() - 1, 1)
        }
    }

    pub fn remove(&mut self, handle: Handle) -> Option<T> {
        let (index, generation) = handle.parts()?;
        let slot = self.slots.get_mut(index)?;
        if slot.generation != generation || slot.value.is_none() {
            return None;
        }
        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.free.push(index);
        slot.value.take()
    }

    pub fn get(&self, handle: Handle) -> Option<&T> {
        let (index, generation) = handle.parts()?;
        let slot = self.slots.get(index)?;
        if slot.generation == generation { slot.value.as_ref() } else { None }
    }

    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        let (index, generation) = handle.parts()?;
        let slot = self.slots.get_mut(index)?;
        if slot.generation == generation { slot.value.as_mut() } else { None }
    }

    pub fn handles(&self) -> Vec<Handle> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.value.is_some())
            .map(|(i, s)| Handle::new(i, s.generation))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_handles_are_rejected() {
        let mut slots = Slots::default();
        let a = slots.insert("a");
        assert_eq!(slots.get(a), Some(&"a"));
        assert_eq!(slots.remove(a), Some("a"));
        let b = slots.insert("b"); // reuses the slot
        assert_ne!(a, b);
        assert_eq!(slots.get(a), None);
        assert_eq!(slots.get(b), Some(&"b"));
        assert_eq!(slots.get(Handle::NULL), None);
        assert_eq!(slots.remove(a), None);
    }
}
