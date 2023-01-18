use core::{
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    sync::atomic::AtomicU32,
};

use alloc::collections::btree_map::Range;
use atomic_refcell::{AtomicRef, AtomicRefMut};

use crate::{
    archetype::{Archetype, Cell, Slot},
    buffer::ComponentBuffer,
    dummy,
    entity::EntityKind,
    filter::{WithRelation, WithoutRelation},
    Component, ComponentInfo, ComponentKey, ComponentValue, Entity,
};

/// Relation helper trait
pub trait RelationExt<T>
where
    T: ComponentValue,
{
    /// Returns the relation id
    fn id(&self) -> Entity;
    /// Instantiate the relation
    fn of(&self, object: Entity) -> Component<T>;
    /// Construct a new filter yielding entities with this kind of relation
    fn with_relation(self) -> WithRelation;
    /// Construct a new filter yielding entities without this kind of relation
    fn without_relation(self) -> WithoutRelation;
}

impl<T, F> RelationExt<T> for F
where
    F: Fn(Entity) -> Component<T>,
    T: ComponentValue,
{
    fn id(&self) -> Entity {
        (self)(dummy()).id()
    }

    fn of(&self, object: Entity) -> Component<T> {
        (self)(object)
    }

    fn with_relation(self) -> WithRelation {
        let c = self(dummy());
        WithRelation {
            relation: c.id(),
            name: c.name(),
        }
    }

    fn without_relation(self) -> WithoutRelation {
        let c = self(dummy());
        WithoutRelation {
            relation: c.id(),
            name: c.name(),
        }
    }
}

/// Represents a relation which can connect to entities
pub struct Relation<T> {
    id: Entity,
    name: &'static str,
    meta: fn(ComponentInfo) -> ComponentBuffer,
    marker: PhantomData<T>,
}

impl<T> Eq for Relation<T> {}

impl<T> PartialEq for Relation<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Copy for Relation<T> {}

impl<T> Clone for Relation<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name,
            meta: self.meta,
            marker: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Relation<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Relation").field("id", &self.id).finish()
    }
}

impl<T> Display for Relation<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.name, self.id)
    }
}

impl<T> Relation<T>
where
    T: ComponentValue,
{
    pub(crate) fn from_id(
        id: Entity,
        name: &'static str,
        meta: fn(ComponentInfo) -> ComponentBuffer,
    ) -> Self {
        Self {
            id,
            name,
            meta,
            marker: PhantomData,
        }
    }

    #[doc(hidden)]
    pub fn static_init(
        id: &AtomicU32,
        kind: EntityKind,
        name: &'static str,
        meta: fn(ComponentInfo) -> ComponentBuffer,
    ) -> Self {
        let id = Entity::static_init(id, kind);

        // Safety
        // The id is new
        Self {
            id,
            name,
            meta,
            marker: PhantomData,
        }
    }

    /// Returns the relation name
    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl<T: ComponentValue> RelationExt<T> for Relation<T> {
    fn id(&self) -> Entity {
        self.id
    }

    fn of(&self, object: Entity) -> Component<T> {
        Component::from_raw_parts(
            ComponentKey::new(self.id, Some(object)),
            self.name,
            self.meta,
        )
    }

    #[inline]
    fn with_relation(self) -> WithRelation {
        WithRelation {
            relation: self.id(),
            name: self.name(),
        }
    }

    #[inline]
    fn without_relation(self) -> WithoutRelation {
        WithoutRelation {
            relation: self.id(),
            name: self.name(),
        }
    }
}

/// Allows to iterate all relations of a specific type for an entity
pub struct RelationIter<'a, T> {
    cells: Range<'a, ComponentKey, Cell>,
    slot: Slot,
    marker: PhantomData<T>,
}

impl<'a, T: ComponentValue> RelationIter<'a, T> {
    pub(crate) fn new(relation: impl RelationExt<T>, arch: &'a Archetype, slot: Slot) -> Self {
        let relation = relation.id();
        Self {
            cells: arch.cells().range(
                ComponentKey::new(relation, Some(Entity::MIN))
                    ..=ComponentKey::new(relation, Some(Entity::MAX)),
            ),
            slot,
            marker: PhantomData,
        }
    }
}

impl<'a, T> Iterator for RelationIter<'a, T>
where
    T: ComponentValue,
{
    type Item = (Entity, AtomicRef<'a, T>);

    fn next(&mut self) -> Option<Self::Item> {
        let (&key, cell) = self.cells.next()?;
        // Safety: the type matches the relation ext
        Some((key.object().unwrap(), unsafe {
            cell.get::<T>(self.slot).unwrap()
        }))
    }
}

/// See: [crate::RelationIter]
pub struct RelationIterMut<'a, T> {
    cells: Range<'a, ComponentKey, Cell>,
    slot: Slot,
    change_tick: u32,
    marker: PhantomData<T>,
}

impl<'a, T: ComponentValue> RelationIterMut<'a, T> {
    pub(crate) fn new(
        relation: impl RelationExt<T>,
        arch: &'a Archetype,
        slot: Slot,
        change_tick: u32,
    ) -> Self {
        let relation = relation.id();
        Self {
            cells: arch.cells().range(
                ComponentKey::new(relation, Some(Entity::MIN))
                    ..=ComponentKey::new(relation, Some(Entity::MAX)),
            ),
            slot,
            marker: PhantomData,
            change_tick,
        }
    }
}

impl<'a, T> Iterator for RelationIterMut<'a, T>
where
    T: ComponentValue,
{
    type Item = (Entity, AtomicRefMut<'a, T>);

    fn next(&mut self) -> Option<Self::Item> {
        let (&key, cell) = self.cells.next()?;
        // Safety: the type matches the relation ext
        Some((key.object().unwrap(), unsafe {
            cell.get_mut::<T>(self.slot, self.change_tick).unwrap()
        }))
    }
}