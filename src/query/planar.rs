use alloc::vec::Vec;
use core::{iter::Flatten, slice::IterMut};
use smallvec::SmallVec;

use crate::{
    archetype::{ArchetypeId, Slice},
    entity::EntityLocation,
    error::{MissingComponent, Result},
    fetch::{FetchAccessData, PreparedFetch},
    filter::{All, Filtered},
    system::{Access, AccessKind},
    Entity, Error, Fetch, FetchItem, World,
};

use super::{
    borrow::QueryBorrowState, difference::find_missing_components, ArchetypeChunks,
    ArchetypeSearcher, Chunk, PreparedArchetype, QueryStrategy,
};

/// The default linear iteration strategy
#[derive(Clone)]
pub struct Planar {
    pub(super) archetypes: Vec<ArchetypeId>,
}

impl core::fmt::Debug for Planar {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Planar").finish()
    }
}

impl Planar {
    pub(super) fn new() -> Self {
        Self {
            archetypes: Vec::new(),
        }
    }
}

impl Planar {
    // Make sure the archetypes to visit are up to date
    fn update_state<'w, Q: Fetch<'w>, F: Fetch<'w>>(
        world: &crate::World,
        fetch: &Filtered<Q, F>,
        result: &mut Vec<ArchetypeId>,
    ) {
        profile_function!();
        let mut searcher = ArchetypeSearcher::default();
        fetch.searcher(&mut searcher);

        searcher.find_archetypes(&world.archetypes, |arch_id, arch| {
            if !fetch.filter_arch(FetchAccessData {
                world,
                arch,
                arch_id,
            }) {
                return false;
            }

            result.push(arch_id);
            false
        });
    }
}

impl<'w, Q, F> QueryStrategy<'w, Q, F> for Planar
where
    Q: 'w + Fetch<'w>,
    F: 'w + Fetch<'w>,
{
    type Borrow = QueryBorrow<'w, Q, F>;

    fn borrow(&'w mut self, state: QueryBorrowState<'w, Q, F>, dirty: bool) -> Self::Borrow {
        // Make sure the archetypes to visit are up to date
        if dirty {
            self.archetypes.clear();
            Self::update_state(state.world, state.fetch, &mut self.archetypes);
        }

        QueryBorrow {
            prepared: SmallVec::new(),
            archetypes: &self.archetypes,
            state,
        }
    }

    fn access(&self, world: &World, fetch: &Filtered<Q, F>, dst: &mut Vec<Access>) {
        let mut result = Vec::new();
        Self::update_state(world, fetch, &mut result);

        result.iter().for_each(|&arch_id| {
            let arch = world.archetypes.get(arch_id);
            let data = FetchAccessData {
                world,
                arch,
                arch_id,
            };

            fetch.access(data, dst)
        });

        dst.push(Access {
            kind: AccessKind::World,
            mutable: false,
        });
    }
}

/// A lazily prepared query which borrows and hands out chunk iterators for
/// each archetype matched.
///
/// The borrowing is lazy, as such, calling [`QueryBorrow::get`] will only borrow the one required archetype.
/// [`QueryBorrow::iter`] will borrow the components from all archetypes and release them once the prepared query drops.
/// Subsequent calls to iter will use the same borrow.
pub struct QueryBorrow<'w, Q, F = All>
where
    Q: Fetch<'w>,
    F: Fetch<'w>,
{
    prepared: SmallVec<[PreparedArchetype<'w, Q::Prepared, F::Prepared>; 8]>,
    archetypes: &'w [ArchetypeId],
    state: QueryBorrowState<'w, Q, F>,
}

impl<'w, 'q, Q, F> IntoIterator for &'q mut QueryBorrow<'w, Q, F>
where
    Q: Fetch<'w>,
    F: Fetch<'w>,
    'w: 'q,
{
    type Item = <Q::Prepared as PreparedFetch<'q>>::Item;

    type IntoIter = QueryIter<'w, 'q, Q, F>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'w, Q, F> QueryBorrow<'w, Q, F>
where
    Q: Fetch<'w>,
    F: Fetch<'w>,
{
    /// Iterate all items matched by query and filter.
    #[inline]
    pub fn iter<'q>(&'q mut self) -> QueryIter<'w, 'q, Q, F>
    where
        'w: 'q,
    {
        QueryIter {
            iter: self.iter_batched().flatten(),
        }
    }

    /// Returns the first item
    pub fn first(&mut self) -> Option<<Q as FetchItem<'_>>::Item> {
        self.iter().next()
    }

    /// Iterate all items matched by query and filter.
    pub fn iter_batched<'q>(&'q mut self) -> BatchedIter<'w, 'q, Q, F>
    where
        'w: 'q,
    {
        // Prepare all archetypes only if it is not already done
        // Clear previous borrows
        if self.prepared.len() != self.archetypes.len() {
            self.clear_borrows();
            self.prepared = self
                .archetypes
                .iter()
                .filter_map(|&arch_id| {
                    let arch = self.state.world.archetypes.get(arch_id);
                    if arch.is_empty() {
                        return None;
                    }

                    self.state.prepare_fetch(arch_id, arch)
                })
                .collect();
        }

        BatchedIter {
            archetypes: self.prepared.iter_mut(),
            current: None,
        }
    }

    /// Execute a closure for each item in the iterator.
    ///
    /// This is more efficient than `.iter().for_each(|v| {})` as the archetypes can be temporarily
    /// borrowed.
    pub fn for_each(&mut self, mut func: impl FnMut(<Q as FetchItem<'_>>::Item)) {
        self.clear_borrows();
        for &arch_id in self.archetypes {
            let arch = self.state.world.archetypes.get(arch_id);
            if arch.is_empty() {
                continue;
            }

            if let Some(mut p) = self.state.prepare_fetch(arch_id, arch) {
                let chunk = p.chunks();

                for item in chunk.flatten() {
                    func(item)
                }
            }
        }
    }

    /// See: [`QueryBorrow::for_each`]
    pub fn try_for_each<E>(
        &mut self,
        mut func: impl FnMut(<Q as FetchItem<'_>>::Item) -> core::result::Result<(), E> + Send + Sync,
    ) -> core::result::Result<(), E> {
        self.clear_borrows();
        for &arch_id in self.archetypes {
            let arch = self.state.world.archetypes.get(arch_id);
            if arch.is_empty() {
                continue;
            }

            if let Some(mut p) = self.state.prepare_fetch(arch_id, arch) {
                let chunk = p.chunks();

                for item in chunk.flatten() {
                    func(item)?;
                }
            }
        }

        Ok(())
    }

    /// Shorthand for:
    /// ```rust,ignore
    /// self.iter_batched()
    ///     .par_bridge()
    ///     .for_each(|v| v.for_each(&func))
    /// ```
    #[cfg(feature = "rayon")]
    pub fn par_for_each(&mut self, func: impl Fn(<Q as FetchItem<'_>>::Item) + Send + Sync)
    where
        Q: Sync,
        Q::Prepared: Send,
        for<'x> <Q::Prepared as PreparedFetch<'x>>::Chunk: Send,
        F: Sync,
        F::Prepared: Send,
    {
        use rayon::prelude::{ParallelBridge, ParallelIterator};

        self.iter_batched()
            .par_bridge()
            .for_each(|batch| batch.for_each(&func))
    }

    /// Release all borrowed archetypes
    #[inline]
    pub fn clear_borrows(&mut self) {
        self.prepared.clear()
    }

    /// Consumes the iterator and returns the number of entities visited.
    /// Faster than `self.iter().count()`
    pub fn count<'q>(&'q mut self) -> usize
    where
        'w: 'q,
    {
        self.iter_batched().map(|v| v.slots().len()).sum()
    }

    fn prepare_archetype(&mut self, arch_id: ArchetypeId) -> Option<usize> {
        let prepared = &mut self.prepared;

        if let Some(idx) = prepared.iter().position(|v| v.arch_id == arch_id) {
            Some(idx)
        } else {
            let arch = self.state.world.archetypes.get(arch_id);

            if !self.state.fetch.filter_arch(FetchAccessData {
                world: self.state.world,
                arch,
                arch_id,
            }) {
                return None;
            }

            let fetch = self.state.prepare_fetch(arch_id, arch)?;

            // let arch_id = *self.archetypes.iter().find(|&&v| v == arch_id)?;

            prepared.push(fetch);

            Some(prepared.len() - 1)
        }
    }

    /// Get the fetch items for an entity.
    pub fn get(&mut self, id: Entity) -> Result<<Q::Prepared as PreparedFetch>::Item> {
        let EntityLocation { arch_id, slot } = self.state.world.location(id)?;

        let idx =
            self.prepare_archetype(arch_id).ok_or_else(|| {
                match find_missing_components(self.state.fetch, arch_id, self.state.world).next() {
                    Some(missing) => {
                        Error::MissingComponent(MissingComponent { id, desc: missing })
                    }
                    None => Error::DoesNotMatch(id),
                }
            })?;

        // Since `self` is a mutable references the borrow checker
        // guarantees this borrow is unique
        let p = &mut self.prepared[idx];
        // Safety: &mut self
        let mut chunk = unsafe {
            p.create_chunk(Slice::single(slot))
                .ok_or(Error::Filtered(id))?
        };

        let item = chunk.next().unwrap();

        Ok(item)
    }
}

/// The query iterator
pub struct QueryIter<'w, 'q, Q, F>
where
    Q: Fetch<'w>,
    F: Fetch<'w>,
{
    iter: Flatten<BatchedIter<'w, 'q, Q, F>>,
}

impl<'w, 'q, Q, F> Iterator for QueryIter<'w, 'q, Q, F>
where
    Q: Fetch<'w>,
    F: Fetch<'w>,
    'w: 'q,
{
    type Item = <Q::Prepared as PreparedFetch<'q>>::Item;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

// struct SlicePtrIter<T> {
//     ptr: *mut T,
//     count: usize,
// }

// impl<T> SlicePtrIter<T> {
//     fn new(slice: *mut [T]) -> Self {
//         Self {
//             ptr: slice.as_mut_ptr(),
//             count: slice.len,
//         }
//     }
// }

// impl<T> Iterator for SlicePtrIter<T> {
//     type Item = *mut T;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.count == 0 {
//             return None;
//         }

//         self.count -= 1;
//         let old = self.ptr;
//         unsafe {
//             self.ptr = self.ptr.add(1);
//         }
//         Some(old)
//     }
// }

/// An iterator which yields disjoint continuous slices for each matched archetype
/// and filter predicate.
pub struct BatchedIter<'w, 'q, Q, F>
where
    Q: Fetch<'w>,
    F: Fetch<'w>,
    'w: 'q,
{
    pub(crate) archetypes: IterMut<'q, PreparedArchetype<'w, Q::Prepared, F::Prepared>>,
    pub(crate) current: Option<ArchetypeChunks<'q, Q::Prepared, F::Prepared>>,
}

/// Iterates over archetypes, yielding batches
impl<'w, 'q, Q, F> BatchedIter<'w, 'q, Q, F>
where
    Q: Fetch<'w>,
    F: Fetch<'w>,
    'w: 'q,
{
    pub(crate) fn new(
        archetypes: IterMut<'q, PreparedArchetype<'w, Q::Prepared, F::Prepared>>,
    ) -> Self {
        Self {
            archetypes,
            current: None,
        }
    }
}

impl<'w, 'q, Q, F> Iterator for BatchedIter<'w, 'q, Q, F>
where
    Q: Fetch<'w>,
    F: Fetch<'w>,
    'w: 'q,
{
    type Item = Chunk<'q, Q::Prepared>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(chunk) = self.current.as_mut() {
                if let item @ Some(..) = chunk.next() {
                    return item;
                }
            }

            let p = unsafe {
                &mut *(self.archetypes.next()?
                    as *mut PreparedArchetype<'w, Q::Prepared, F::Prepared>)
            };

            self.current = Some(p.chunks());
        }
    }
}
