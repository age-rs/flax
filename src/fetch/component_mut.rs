use core::fmt::{self, Formatter};

use alloc::vec;
use alloc::vec::Vec;
use atomic_refcell::AtomicRefMut;

use crate::{
    archetype::{Archetype, Cell, CellMutGuard, Change, Changes, Slice, Slot},
    events::{EventData, EventKind},
    system::{Access, AccessKind},
    Component, ComponentValue, Entity, Fetch, FetchItem,
};

use super::{FetchAccessData, FetchPrepareData, PreparedFetch};

#[doc(hidden)]
pub struct WriteComponent<'a, T> {
    storage: AtomicRefMut<'a, [T]>,
    changes: AtomicRefMut<'a, Changes>,
    cell: &'a Cell,
    ids: &'a [Entity],
    tick: u32,
}

#[derive(Debug, Clone)]
/// Mutable component fetch
/// See [crate::Component::as_mut]
pub struct Mutable<T: ComponentValue>(pub(crate) Component<T>);

impl<'w, T> Fetch<'w> for Mutable<T>
where
    T: ComponentValue,
{
    const MUTABLE: bool = true;

    type Prepared = WriteComponent<'w, T>;

    #[inline]
    fn prepare(&self, data: FetchPrepareData<'w>) -> Option<Self::Prepared> {
        let CellMutGuard {
            storage,
            changes,
            cell,
            ids,
            tick,
        } = data.arch.borrow_mut(self.0, data.new_tick)?;

        Some(WriteComponent {
            storage,
            changes,
            cell,
            ids,
            tick,
        })
    }

    #[inline]
    fn filter_arch(&self, arch: &Archetype) -> bool {
        arch.has(self.0.key())
    }

    #[inline]
    fn access(&self, data: FetchAccessData) -> Vec<Access> {
        if data.arch.has(self.0.key()) {
            vec![
                Access {
                    kind: AccessKind::Archetype {
                        id: data.arch_id,
                        component: self.0.key(),
                    },
                    mutable: true,
                },
                Access {
                    kind: AccessKind::ChangeEvent {
                        id: data.arch_id,
                        component: self.0.key(),
                    },
                    mutable: true,
                },
            ]
        } else {
            vec![]
        }
    }

    fn describe(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("mut ")?;
        f.write_str(self.0.name())
    }

    fn searcher(&self, searcher: &mut crate::ArchetypeSearcher) {
        searcher.add_required(self.0.key())
    }
}

impl<'q, T: ComponentValue> FetchItem<'q> for Mutable<T> {
    type Item = &'q mut T;
}

impl<'q, 'w, T: 'q> PreparedFetch<'q> for WriteComponent<'w, T> {
    type Item = &'q mut T;

    #[inline(always)]
    unsafe fn fetch(&'q mut self, slot: Slot) -> Self::Item {
        // Perform a reborrow
        // Cast from a immutable to a mutable borrow as all calls to this
        // function are guaranteed to be disjoint
        unsafe { &mut *(self.storage.get_unchecked_mut(slot) as *mut T) }
    }

    fn set_visited(&mut self, slots: Slice) {
        let event = EventData {
            ids: &self.ids[slots.as_range()],
            key: self.cell.info().key,
            kind: EventKind::Modified,
        };

        for handler in self.cell.subscribers.iter() {
            handler.on_event(&event)
        }

        self.changes
            .set_modified_if_tracking(Change::new(slots, self.tick));
    }
}
