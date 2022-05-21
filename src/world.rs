use atomic_refcell::{AtomicRef, AtomicRefMut};

use crate::{
    archetype::{Archetype, ArchetypeId, ComponentInfo},
    entity::{EntityLocation, EntityStore},
    Component, ComponentId, ComponentValue, Entity,
};

pub struct World {
    entities: EntityStore,
    archetypes: Vec<Archetype>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: EntityStore::new(),
            archetypes: vec![Archetype::empty()],
        }
    }

    /// Get the archetype which has `components`.
    /// `components` must be sorted.
    pub fn find_archetype(
        &self,
        root: ArchetypeId,
        mut components: &[ComponentId],
    ) -> Option<&Archetype> {
        let mut cursor = root;

        while let [head, tail @ ..] = components {
            let next = self.archetypes[cursor as usize].edge_to(*head)?;
            cursor = next;
            components = tail;
        }

        Some(&self.archetypes[cursor as usize])
    }

    /// Get the archetype which has `components`.
    /// `components` must be sorted.
    pub fn fetch_archetype(
        &mut self,
        root: ArchetypeId,
        mut components: &[ComponentInfo],
    ) -> (ArchetypeId, &mut Archetype) {
        let mut cursor = root;

        let all = components;
        let mut i = 0;

        while let [head, tail @ ..] = components {
            let id = self.archetypes.len() as u32;
            let cur = &mut self.archetypes[cursor as usize];
            cursor = match cur.edge_to(head.id) {
                Some(id) => id,
                None => {
                    // Create archetype
                    eprintln!(
                        "Creating new archetype {:?} => {}\n {:#?}",
                        cur.components().last().map(|v| v.name),
                        head.name,
                        &all[..=i]
                    );
                    let mut new = Archetype::new(all[..=i].to_vec());

                    cur.add_edge_to(&mut new, id, cursor, head.id);

                    self.archetypes.push(new);
                    id
                }
            };
            components = tail;

            i += 1;
        }

        (cursor, &mut self.archetypes[cursor as usize])
    }

    /// Spawn a new empty entity
    pub fn spawn(&mut self) -> Entity {
        // Place at root
        let id = self.entities.spawn(EntityLocation::default());
        // This is safe as `root` does not contain any components
        let slot = unsafe { self.archetype_mut(0).allocate(id) };
        self.entities.get_mut(id).unwrap().slot = slot;
        id
    }

    /// Access an archetype by id
    pub fn archetype(&self, id: ArchetypeId) -> &Archetype {
        &self.archetypes[id as usize]
    }

    /// Access an archetype by id
    pub fn archetype_mut(&mut self, id: ArchetypeId) -> &mut Archetype {
        &mut self.archetypes[id as usize]
    }

    pub fn insert<T: ComponentValue>(&mut self, id: Entity, component: Component<T>, mut value: T) {
        let &EntityLocation {
            archetype: src_id,
            slot,
        } = self.entities.get(id).unwrap();
        let src = self.archetype(src_id);

        let components = src.components();
        let pivot = components
            .iter()
            .take_while(|v| v.id < component.id())
            .count();

        // Split the components
        // A B C [new] D E F
        let left = &components[0..pivot];
        let right = &components[pivot..];
        let component_info = component.info();

        let mut components = Vec::with_capacity(left.len() + 1 + right.len());
        components.extend_from_slice(left);
        components.push(component_info);
        components.extend_from_slice(right);

        // assert in order

        {
            let mut sorted = components.clone();
            sorted.sort_by_key(|v| v.id);
            assert_eq!(sorted, components);
        }

        let (dst_id, _) = self.fetch_archetype(0, &components);
        // let src = self.archetype_mut(src_id);

        unsafe {
            assert_ne!(src_id, dst_id);
            // Borrow disjoint
            let src =
                &mut *((&self.archetypes[src_id as usize]) as *const Archetype as *mut Archetype);
            let dst =
                &mut *((&self.archetypes[dst_id as usize]) as *const Archetype as *mut Archetype);

            let (dst_slot, swapped) = src.move_to(dst, slot);

            // Insert the missing component
            dst.put_dyn(dst_slot, &component_info, &mut value as *mut T as *mut u8)
                .expect("Insert should not fail");

            assert_eq!(dst.entity(dst_slot), Some(id));
            if let Some(swapped) = swapped {
                // The last entity in src was moved into the slot occupied by id
                eprintln!("Relocating entity");
                self.entities
                    .get_mut(swapped)
                    .expect("Invalid entity id")
                    .slot = slot;
            }

            *self.entities.get_mut(id).expect("Entity is not valid") = EntityLocation {
                slot: dst_slot,
                archetype: dst_id,
            };
        }
    }

    /// Randomly access an entity's component.
    pub fn get<T: ComponentValue>(
        &self,
        id: Entity,
        component: Component<T>,
    ) -> Option<AtomicRef<T>> {
        let loc = self.entities.get(id)?;

        self.archetypes[loc.archetype as usize].get(loc.slot, component)
    }

    /// Randomly access an entity's component.
    pub fn get_mut<T: ComponentValue>(
        &self,
        id: Entity,
        component: Component<T>,
    ) -> Option<AtomicRefMut<T>> {
        let loc = self.entities.get(id)?;

        self.archetypes[loc.archetype as usize].get_mut(loc.slot, component)
    }

    /// Returns true if the entity has the specified component.
    /// Returns false if
    pub fn has<T: ComponentValue>(&self, id: Entity, component: Component<T>) -> bool {
        let loc = self.entities.get(id);
        if let Some(loc) = loc {
            self.archetype(loc.archetype).has(component.id())
        } else {
            false
        }
    }

    /// Despawns an entity
    pub fn despawn(&mut self, id: Entity) {
        self.entities.despawn(id)
    }

    /// Returns true if the entity is still alive
    pub fn is_alive(&self, id: Entity) -> bool {
        self.entities.is_alive(id)
    }

    pub(crate) fn archetypes(&self) -> impl Iterator<Item = (ArchetypeId, &Archetype)> {
        self.archetypes
            .iter()
            .enumerate()
            .map(|(i, v)| (i as ArchetypeId, v))
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    component! {
        a: i32,
        b: f32,
        c: String,
        d: Vec<u32>,
    }

    #[test]
    fn world_archetype_graph() {
        let mut world = World::new();

        // () -> (a) -> (ab) -> (abc)
        let (_, archetype) = world.fetch_archetype(0, &[a().info(), b().info(), c().info()]);
        assert!(!archetype.has(d().id()));
        assert!(archetype.has(a().id()));
        assert!(archetype.has(b().id()));

        // dbg!(&world.archetypes);

        // () -> (a) -> (ab) -> (abc)
        //                   -> (abd)
        let (_, archetype) = world.fetch_archetype(0, &[a().info(), b().info(), d().info()]);
        assert!(archetype.has(d().id()));
        assert!(!archetype.has(c().id()));
    }

    #[test]
    fn insert() {
        let mut world = World::new();
        let id = world.spawn();

        world.insert(id, a(), 65);

        assert_eq!(world.get(id, a()).as_deref(), Some(&65));
        assert_eq!(world.get(id, b()).as_deref(), None);

        world.insert(id, b(), 0.3);

        eprintln!("a: {}, b: {}, c: {}, id: {}", a(), a(), c(), id);

        assert_eq!(world.get(id, a()).as_deref(), Some(&65));
        assert_eq!(world.get(id, b()).as_deref(), Some(&0.3));
        assert_eq!(world.has(id, c()), false);
    }

    #[test]
    fn concurrent_borrow() {
        let mut world = World::new();
        let id1 = world.spawn();
        let id2 = world.spawn();

        world.insert(id1, a(), 40);

        world.insert(id2, b(), 4.3);

        // Borrow a
        let id_a = world.get(id1, a()).unwrap();
        assert_eq!(*id_a, 40);
        // Borrow b uniquely while a is in scope
        let mut id2_b = world.get_mut(id2, b()).unwrap();

        *id2_b = 3.21;

        assert_eq!(*id_a, 40);

        // Borrow another component on an entity with a mutably borrowed
        // **other** component.
        assert_eq!(world.get(id2, a()).as_deref(), None);
    }
}
