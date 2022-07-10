use flax::{component, util::TupleCloned, CmpExt, EntityBuilder, FetchExt, Query, World};
use itertools::Itertools;

#[test]
fn query_change() {
    component! {
        name: String,
        health: f32,
        pos: (f32, f32),
        // Distance from origin
        distance: f32,
    }

    let mut world = World::new();
    EntityBuilder::new()
        .set(name(), "A".into())
        .set(health(), 50.0)
        .set(pos(), (3.0, 1.0))
        .set_default(distance())
        .spawn(&mut world);

    EntityBuilder::new()
        .set(name(), "B".into())
        .set(health(), 89.0)
        .set(pos(), (3.0, 8.0))
        .set_default(distance())
        .spawn(&mut world);

    EntityBuilder::new()
        .set(name(), "C".into())
        .set(health(), 30.0)
        .set(pos(), (3.0, 8.0))
        .set_default(distance())
        .spawn(&mut world);

    EntityBuilder::new()
        .set(name(), "D".into())
        .set(health(), 70.0)
        .set(pos(), (3.0, 8.0))
        .set_default(distance())
        .spawn(&mut world);

    // Only those strong enough shall move
    let mut move_alive = Query::new((name(), pos().as_mut())).filter(health().gt(40.0));
    let mut consumer = Query::new((name(), pos(), distance().as_mut())).filter(pos().modified());

    // Ignore spawn changes to only capture `move_alive`
    consumer.ignore_changes(&world);

    let moved = move_alive
        .prepare(&world)
        .iter()
        .map(|(name, pos)| {
            pos.0 += 1.0;
            pos.1 *= 0.99;
            name.to_string()
        })
        .sorted()
        .collect_vec();

    assert_eq!(moved, ["A", "B", "D"]);

    dbg!(world.change_tick(), &consumer);

    let consumed = consumer
        .prepare(&world)
        .iter()
        .map(|(name, pos, distance)| {
            *distance = (pos.0 * pos.0 + pos.1 * pos.1).sqrt();
            name.to_string()
        })
        .sorted()
        .collect_vec();

    assert_eq!(consumed, ["A", "B", "D"]);
    // Everything which is alive will move a bit
}

#[test]
fn query_opt() {
    component! {
        name: String,
        mass: f32,
        vel: f32,
        status_effect: String,
    }

    let mut world = World::new();
    EntityBuilder::new()
        .set(name(), "Alyx".to_string())
        .set(mass(), 70.0)
        .set(vel(), 1.0)
        .set(status_effect(), "Neurotoxin".to_string())
        .spawn(&mut world);

    EntityBuilder::new()
        .set(name(), "Gordon".to_string())
        .set(mass(), 95.0)
        .set(vel(), 1.5)
        .spawn(&mut world);

    EntityBuilder::new()
        .set(name(), "Citadel".to_string())
        .set(mass(), 1e9)
        .spawn(&mut world);

    let mut query = Query::new((name(), mass(), vel().opt_or_default()));

    let items = query
        .prepare(&world)
        .iter()
        .sorted_by_key(|v| v.0)
        .map(|v| v.cloned())
        .collect_vec();

    assert_eq!(
        items,
        [
            ("Alyx".to_string(), 70.0, 1.0),
            ("Citadel".to_string(), 1e9, 0.0),
            ("Gordon".to_string(), 95.0, 1.5)
        ]
    );

    let mut query = Query::new((name(), status_effect().opt()));
    let mut query = query.prepare(&world);
    let items = query.iter().sorted().collect_vec();

    assert_eq!(
        items,
        [
            (&"Alyx".to_string(), Some(&"Neurotoxin".to_string())),
            (&"Citadel".to_string(), None),
            (&"Gordon".to_string(), None),
        ]
    );
}
