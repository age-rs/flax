use std::{thread::sleep, time::Duration};

use flax::components::name;

#[test]
fn access() {
    use flax::*;
    component! {
        health: f32,
        weapon: &'static str,
        blue_team: (),
        red_team: (),
        support_of(id): (),
    }

    let mut world = World::new();

    // Spectator
    let spectator = Entity::builder()
        .set(name(), "spectator".into())
        .set(health(), 100.0)
        .set(weapon(), "bow")
        .spawn(&mut world);

    let _blue1 = Entity::builder()
        .set(name(), "blue1".into())
        .set(health(), 100.0)
        .set(weapon(), "Rifle")
        .tag(blue_team())
        .spawn(&mut world);

    let red1 = Entity::builder()
        .set(name(), "red1".into())
        .set(health(), 100.0)
        .set(weapon(), "Wrench")
        .tag(red_team())
        .spawn(&mut world);

    // Like a turret
    Entity::builder()
        .set(name(), "turret".into())
        .set(health(), 100.0)
        .tag(red_team())
        .tag(support_of(red1))
        .spawn(&mut world);

    let weapons = System::builder()
        .with_name("weapons")
        .with_query(Query::new(weapon()))
        .build(|mut q: QueryBorrow<Component<&'static str>>| {
            let weapons: Vec<_> = q.iter().collect();
            eprintln!("Weapons in use: {weapons:?}");
        })
        .boxed();

    let names = System::builder()
        .with_name("names")
        .with_query(Query::new(name()))
        .build(|mut q: QueryBorrow<Component<String>>| {
            let names: Vec<_> = q.iter().collect();
            eprintln!("names in use: {names:?}");
        })
        .boxed();

    let regen_system = System::builder()
        .with_name("regen_system")
        .with_query(Query::new(health().as_mut()))
        .try_for_each(|v| {
            *v = (*v + 10.0).min(100.0);
            anyhow::Ok(())
        })
        .boxed();

    let blue_system = System::builder()
        .with_name("blue_system")
        .with_query(Query::new(weapon().as_mut()).with_filter(blue_team().with() & health().gt(0.0)))
        .for_each(|_v| {
            sleep(Duration::from_millis(100));
            // here be logic
        })
        .boxed();

    let red_system = System::builder()
        .with_name("red_system")
        .with_query(Query::new(weapon().as_mut()).with_filter(red_team().with() & health().gt(0.0)))
        .for_each(|_v| {
            sleep(Duration::from_millis(100));
            // here be logic
        })
        .boxed();

    let stats_system = System::builder()
        .with_name("stats")
        .with_query(Query::new((weapon(), health())).with_filter(health().gt(0.0)))
        .for_each(|(weapon, health)| eprintln!("player using {weapon} is alive {health}"))
        .boxed();

    let mut schedule = Schedule::from([
        regen_system,
        weapons,
        blue_system,
        red_system,
        stats_system,
        names,
    ]);

    assert_eq!(
        schedule.batch_info(&world).to_names(),
        [
            &["regen_system", "weapons", "names"][..],
            &["blue_system", "red_system"],
            &["stats"]
        ]
    );

    world.set(spectator, blue_team(), ()).unwrap();

    assert_eq!(
        schedule.batch_info(&world).to_names(),
        [
            &["regen_system", "weapons", "names"][..],
            &["blue_system", "red_system"],
            &["stats"]
        ]
    );

    // Now on both teams, can no longer parallelize
    world.set(spectator, red_team(), ()).unwrap();

    assert_eq!(
        schedule.batch_info(&world).to_names(),
        [
            &["regen_system", "weapons", "names"][..],
            &["blue_system"],
            &["red_system"],
            &["stats"],
        ]
    );

    world.remove(spectator, weapon()).unwrap();

    assert_eq!(
        schedule.batch_info(&world).to_names(),
        [
            &["regen_system", "weapons", "names"][..],
            &["blue_system"],
            &["red_system"],
            &["stats"]
        ]
    );

    world.prune_archetypes();

    assert_eq!(
        schedule.batch_info(&world).to_names(),
        [
            &["regen_system", "weapons", "names"][..],
            &["blue_system", "red_system"],
            &["stats"]
        ]
    );
}
