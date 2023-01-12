use core::{iter::Flatten, slice::IterMut};

use crate::{
    archetype::{Slice, Slot},
    fetch::{FetchPrepareData, PreparedFetch},
    filter::{FilterIter, PreparedFilter, RefFilter},
    Archetype, Entity, Fetch, Filter, World,
};

use super::{FilterWithFetch, PreparedArchetype};

/// Iterates over a chunk of entities, specified by a predicate.
/// In essence, this is the unflattened version of [crate::QueryIter].
pub struct Batch<'q, Q> {
    arch: &'q Archetype,
    fetch: &'q mut Q,
    pos: Slot,
    end: Slot,
}

impl<'q, Q> core::fmt::Debug for Batch<'q, Q> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Batch")
            .field("pos", &self.pos)
            .field("end", &self.end)
            .finish()
    }
}

impl<'q, Q> Batch<'q, Q> {
    pub(crate) fn new(arch: &'q Archetype, fetch: &'q mut Q, slice: Slice) -> Self {
        Self {
            arch,
            fetch,
            pos: slice.start,
            end: slice.end,
        }
    }

    pub(crate) fn slots(&self) -> Slice {
        Slice::new(self.pos, self.end)
    }

    /// Returns the archetype for this batch.
    /// **Note**: The borrow of the fetch is still held and may result in borrow
    /// errors.
    pub fn arch(&self) -> &Archetype {
        self.arch
    }

    /// Returns the number of items which would be yielded by this batch
    pub fn len(&self) -> usize {
        self.slots().len()
    }

    /// Returns true if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.slots().is_empty()
    }
}

impl<'q, Q> Iterator for Batch<'q, Q>
where
    Q: PreparedFetch<'q>,
{
    type Item = Q::Item;

    fn next(&mut self) -> Option<Q::Item> {
        if self.pos == self.end {
            None
        } else {
            let fetch = unsafe { &mut *(self.fetch as *mut Q) };
            let item = unsafe { fetch.fetch(self.pos) };
            self.pos += 1;
            Some(item)
        }
    }
}

impl<'q, Q> Batch<'q, Q>
where
    Q: PreparedFetch<'q>,
{
    pub(crate) fn next_with_id(&mut self) -> Option<(Entity, Q::Item)> {
        if self.pos == self.end {
            None
        } else {
            let fetch = unsafe { &mut *(self.fetch as *mut Q) };
            let item = unsafe { fetch.fetch(self.pos) };
            let id = self.arch.entities[self.pos];
            self.pos += 1;
            Some((id, item))
        }
    }
}
/// An iterator over a single archetype which returns chunks.
/// The chunk size is determined by the largest continuous matched entities for
/// filters.
pub struct ArchetypeChunks<'q, Q, F> {
    pub(crate) arch: &'q Archetype,
    pub(crate) fetch: &'q mut Q,
    pub(crate) filter: FilterIter<F>,
    pub(crate) new_tick: u32,
}

impl<'q, Q, F> Iterator for ArchetypeChunks<'q, Q, F>
where
    Q: PreparedFetch<'q>,
    F: PreparedFilter,
{
    type Item = Batch<'q, Q>;

    fn next(&mut self) -> Option<Self::Item> {
        // Get the next chunk
        let chunk = self.filter.next();
        let chunk = chunk?;

        // Fetch will never change and all calls are disjoint
        let fetch = unsafe { &mut *(self.fetch as *mut Q) };

        // Set the chunk as visited
        unsafe { fetch.set_visited(chunk, self.new_tick) }
        let chunk = Batch::new(self.arch, fetch, chunk);

        Some(chunk)
    }
}

/// The query iterator
pub struct QueryIter<'q, 'w, Q, F>
where
    Q: Fetch<'w>,
    F: Filter<'q>,
{
    iter: Flatten<BatchedIter<'q, 'w, Q, F>>,
}

impl<'q, 'w, Q, F> QueryIter<'q, 'w, Q, F>
where
    Q: Fetch<'w>,
    F: Filter<'q>,
{
    #[inline(always)]
    pub(crate) fn new(iter: BatchedIter<'q, 'w, Q, F>) -> Self {
        Self {
            iter: iter.flatten(),
            // current: None,
        }
    }
}

impl<'w, 'q, Q, F> Iterator for QueryIter<'q, 'w, Q, F>
where
    Q: Fetch<'w>,
    F: Filter<'q>,
    'w: 'q,
{
    type Item = <Q::Prepared as PreparedFetch<'q>>::Item;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// An iterator which yields disjoint continuous slices for each matched archetype
/// and filter predicate.
pub struct BatchedIter<'q, 'w, Q, F>
where
    Q: Fetch<'w>,
    F: Filter<'q>,
    'w: 'q,
{
    world: &'w World,
    pub(crate) old_tick: u32,
    pub(crate) new_tick: u32,
    pub(crate) filter: &'q FilterWithFetch<RefFilter<'w, F>, Q::Filter>,
    pub(crate) archetypes: IterMut<'q, PreparedArchetype<'w, Q::Prepared>>,
    pub(crate) current: Option<
        ArchetypeChunks<'q, Q::Prepared, <FilterWithFetch<F, Q::Filter> as Filter<'q>>::Prepared>,
    >,
}

impl<'q, 'w, Q, F> BatchedIter<'q, 'w, Q, F>
where
    Q: Fetch<'w>,
    F: Filter<'q>,
{
    pub(super) fn new(
        world: &'w World,
        old_tick: u32,
        new_tick: u32,
        filter: &'q FilterWithFetch<RefFilter<'w, F>, Q::Filter>,
        archetypes: IterMut<'q, PreparedArchetype<'w, Q::Prepared>>,
    ) -> Self {
        Self {
            world,
            old_tick,
            new_tick,
            filter,
            archetypes,
            current: None,
        }
    }
}

impl<'w, 'q, Q, F> Iterator for BatchedIter<'q, 'w, Q, F>
where
    Q: Fetch<'w>,
    F: Filter<'q>,
    'w: 'q,
{
    type Item = Batch<'q, Q::Prepared>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(chunk) = self.current.as_mut() {
                if let item @ Some(..) = chunk.next() {
                    return item;
                }
            }

            let PreparedArchetype {
                arch,
                fetch,
                arch_id,
            } = self.archetypes.next()?;

            let filter = FilterIter::new(
                arch.slots(),
                self.filter.prepare(
                    FetchPrepareData {
                        world: self.world,
                        arch,
                        arch_id: *arch_id,
                    },
                    self.old_tick,
                ),
            );

            let chunk = ArchetypeChunks {
                arch,
                fetch,
                filter,
                new_tick: self.new_tick,
            };

            self.current = Some(chunk);
        }
    }
}
