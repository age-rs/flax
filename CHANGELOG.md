# Changelog

All notable changes to this project will be documented in this file.

## [0.7.1] - 2024-05-02

### Features

- Try_for_each system
- Entity id sink adapter
- `relations_like_mut`

### Bug Fixes

- Implement Error for MissingComponent
- EntityRef::query no longer requires self lifetime

## [0.7.0] - 2024-04-01

### Features

- RelationExt::nth_relation
- FetchExt::filtered
- Implement RandomFetch for NthRelation
- Add name shorthand for entity ref
- Puffin integration
- Add set_opt to `EntityRefMut` and `CommandBuffer`
- Allow accessing world in EntityRef

### Bug Fixes

- Nth relation filtering
- Export Storage
- Nostd build
- Despawn_children performance
- Make mutable easier to use in modification queries
- Performance of tree despawns for high number of archetypes
- Improve dfs performance
- Panic for DfsBorrow::iter_from when there are no relations
- Infinite memory growth in command buffer
- Incorrect changed tranform for mutable
- Filtering for mutable and optional queries
- Don't generate transformed types if there are no transform attrs

### Miscellaneous Tasks

- Profile system execution
- Profile more heavy methods
- Remove coverage due to upstream tarpaulin failure
- Benchmark dfs
- Add transforms to EntityIds
- Update dependencies
- Release

## [0.6.2] - 2024-02-15

### Features

- EntityBuilder::is_empty

### Bug Fixes

- Lazy component buffer in static memory
- Deploy toolchain spec

### Miscellaneous Tasks

- Update ci toolchain
- Release

## [0.6.1] - 2024-02-07

### Bug Fixes

- Update canvas border color
- Conditional and unconditional yield of optional filtered queries
- Preserve field visibility for query items
- Expose `NthRelation`
- Apply commandbuffer in manually run boxed systems
- Stale relations in detached archetypes

### Miscellaneous Tasks

- Release

## [0.6.0] - 2023-10-29

### Features

- Ignore fields in fetch transformation
- Set_missing
- CommandBuffer::set_dedup
- Remove fetch slot indexing and allow acces to world in filters
- Traverse relations in query
- Make entity ids a filter
- QueryOne
- Allow constructing query modifiers in const contexts
- Improve change list removal
- Allow access to storage in event subscription
- Document exclusive relations
- Nth_relation

### Bug Fixes

- Invalid batching when world archetypes are modifed during execution
- Doctest
- [**breaking**] Don't automatically prune archetypes
- Include input lifetime in `SystemContext`
- Miri
- Implement RandomFetch for entity ids
- Source for slot filtering
- Broken link
- Always merge changes on set
- Correctness of `Changes::set` for existing overlaps
- Overlapping slots in change list
- Invalid archetype for a transitive archetype connection
- Clarify associated values for relations
- Clarify value uniqueness
- [**breaking**] Clear up naming with relation target
- Nth_relation access granularity
- Clarify target terms
- Remove erronous cfg guard
- Unused variable
- Debug not implemented for `Type`

### Documentation

- Systems

### Refactor

- [**breaking**] Replace wrapper generics with explicit functions in system builder
- Make `IntoInput` safe

### Testing

- Schedule tuple input

### Miscellaneous Tasks

- Update changelog
- Update README.md
- Implement ExtractDyn for tuples of 4
- Fix warnings
- Cleanup asteroids
- Update toolchain for asteroids
- [**breaking**] Reduce root exports
- Clippy
- Fix doctests
- Document `name` special handling
- Remove debug scaffolding
- Fix no-std tests
- Give asteroids a face lift
- [**breaking**] Remove `removed` filter
- Wording
- Update dependencies

## [0.5.0] - 2023-08-11

### Features

- `as_cloned`
- Automatic archetype pruning
- Implement Debug for EntityRef and EntityRefMut
- Spawn_ref
- Downgrade
- Entry_ref
- Copied
- Entity_ref fetch
- Relation iteration
- Dfs query with change detection
- Support cmp for other queries
- Abstract query strategy
- Planar query strategy
- Move query shorthand methods
- Entity strategy
- Include/exclude components in planar query
- Simplify vtable usage
- Topological query
- Dfs edge values
- Extract `Archetypes`
- Maybe_mut
- Proxy source for fetch items
- Relation source
- Archetype ordering
- Topological query order
- Random tree traversal
- Improved event system
- Query chapter
- Mutually exclusive relation constraints
- Make child_of exclusive
- DfsRoots
- Merge `Dfs` and `DfsRoots`
- Trigger an ICE
- [**breaking**] Feature gate derive
- Improve and clarify schedule reordering
- Better system description
- User supplied context data in schedule execution
- Entity builder component count
- Hierarchy formatting
- Implement display for `Entity` and `EntityRefMut`
- EntityRef::update
- Get copy
- Implement modified transform for tuples
- Use trait to support Union filter for foreign types
- Make derive support generics
- Derive modified transform
- Generic fetch transforms
- Fetch map
- Buffer component writer
- CellMutGuard and CellGuard mapping
- EntityRefMut::set_dedup
- Update_dedup
- Inserted transform
- Make entity errors more specific

### Bug Fixes

- QueryIter perf compared to manual flatten
- Complex type
- [**breaking**] Needless result in EntityRefMut::set
- Component not initialized using set(_dyn)
- Allow returning borrows from EntityRef
- Tests
- Tests
- Query dirty state
- [**breaking**] Logic errors with `filter_arch` and `prepare` returning None
- Allow prepare to arbitrary fail
- Warnings
- Satisfied for dynamic filters
- [**breaking**] Remove spawn_with in favor of entity builder
- Tests
- [**breaking**] Debug => Debuggable
- No-std tests
- Tests
- Unaligned NonNull::dangling
- Dfs recursive reborrowing
- Replace eyre due to maintenance and miri
- Buffer realloc alignment
- Test no_std
- Ignore tarpaulin
- Inlining perf regression
- Derive feature
- Broken MIR by pinning to older version
- #4 broken link
- Export entry
- Unwrap on change filter when missing changes
- Nostd
- Use top-down access construction
- Allow empty systems
- Implement IntoIterator for `&mut TopoBorrow`
- Nostd
- Make QueryBorrow::for_each use FnMut
- [**breaking**] Reduce exports of commonplace names in root
- Doc links
- Ci badge
- Type in README.md
- [**breaking**] Reduce `And` nesting in query filter parameter
- Typos in README.md
- Warnings
- Warnings
- No std tests
- Invalid archetype
- Rename inserted to added
- Use of std
- Source
- Remaining queries
- No-std
- Use of private module

### Documentation

- Fix broken links
- Change detection
- Query
- Traverse and transforms

### Performance

- Make QueryBorrow::for_each lend borrow archetypes
- Borrow clearing

### Refactor

- Archetype change events
- Query archetype searching
- Use vtable for component delegates
- ReadOnlyFetch
- Component buffer
- Simplify writer traits
- Remove set_inner

### Testing

- Clearing
- Entity builder relations
- Replace existing relation on entity using builder

### Miscellaneous Tasks

- Update changelog
- [**breaking**] Make Fetch::match and Filter::match well, match
- Fix lints
- Make `Live Demo` a link
- Remove ComponentValue bound for Component Debug impl
- Make Filter similar to Fetch
- Use default members
- Use `relations_like` in relations fetch
- Make fetch unsafe
- Cleanup fetch trait visiting
- Replace old query with strategy query
- Fix tests for no-std
- Fix warnings
- Fix warnings
- Remove internal duplicate function
- Remove symmetric feature idea
- Update docs
- Reduce dependencies
- Note on eyre
- Tarpaulin action
- Remove eprintln in test
- Remove test_log
- Tarpaulin llvm engine
- Add git-cliff config
- Rename module
- Remove release workflow
- Update git-cliff config
- Less verbose display impls
- Update tynm
- Update syn and cleanup derive macro
- Make codecov informational
- Add asteroids src to README.md
- Force CI run
- Split filters into more modules
- Move union to filter modules
- Attempt to use GAT
- Use fully qualified syntax for derive
- Implement transform for Opt, Cloned, Copied
- Remove adjacent atomic ref cell borrowing
- Use entity slice directly
- Cleanup
- Make set use writer abstraction
- Make set_with use new ComponentWriter
- Cleanup
- Sync readme
- ComponentInfo => ComponentDesc
- Cleanup
- Batch => chunk
- Improve miri speed
- Use advancing ptr
- [**breaking**] Remove redundant AccessKind::ChangeEvent

### Ci

- Cargo nextest
- Fix args
- Git changelog
- Miri job count

## [0.3.2] - 2022-11-09

### Features

- EntityRefMut::retain
- EntityBuilder::set_opt

### Bug Fixes

- Clear not generating removal events for queries
- ChangeSubscriber not working with filter

### Miscellaneous Tasks

- Update changelog

## [0.3.1] - 2022-11-05

### Features

- Filter subscription
- Tokio subscribers
- Extensible event subscription

### Bug Fixes

- Set(_with) not working for reserved entities
- Make EntityIndex primitive
- No-default-features lints
- Blanklines in example
- Doclinks in README

### Refactor

- Archetype change events

### Testing

- Change subscribing
- Subscribe
- Sparse or combinators

### Miscellaneous Tasks

- CHANGELOG.md
- Fix tests
- Simplify internal archetype borrowing api
- Fix no-std
- Fix warnings
- Remove duplicate simpler event_registry
- Doclinks

## [0.3.0] - 2022-10-18

### Features

- Benchmarking
- Batch_size
- Human friendly access info
- Query trie archetype searching
- Row and column serialize benchmarks
- Par_for_each
- No_std
- Rework components and relations
- Concurrently reserve entities
- Asteroids wasm example
- EntityQuery
- Make Query::get use filters
- Require `Filter` to implement bitops
- Make merge_with append to static ids (instead of ignoring and dropping components)

### Bug Fixes

- Ron ident deserialize
- Rename serde module due to crate:serde collision
- Change list remove performance
- Schedule granularity
- Unnecessary checks
- Feature gated benchmarks
- Doctests
- Warnings
- Badge links
- Quasi-quadratic growth strategy
- Whitespace in badges
- Warnings
- No_std tests
- Auto spawn static entities
- Cmds not applied in schedule_seq
- Artefact location
- Dead links
- Feature gate flume due to std requirement
- Asteroids deps
- Spacing
- Use describe rather than requiring debug for filters

### Refactor

- Use a freelist vec instead of inplace linked list

### Testing

- System access and scheduling
- Filter combinators

### Miscellaneous Tasks

- Add guide badge
- Add keywords
- Inline some hot callsites
- Remove tynm
- Fix unused imports with --no-default-features
- Merge deployment of guide and asteroids demo
- Change guide location
- Consistent workflow names
- Use EntityQuery in asteroids
- Remove unneded `fetch::missing`
- [**breaking**] Rename `is_component` => `component_info`
- Cleanup docs
- Make rayon examples use custom thread pool
- Fix doctests

## [0.2.0] - 2022-09-11

### Features

- Change around world access
- Parallel scheduling
- Optional queries
- Entity ref
- Entry like component and entity api
- Standard components
- Component metadata and components
- Implement debug for world
- Batched iteration
- With_world and with_cmd
- Detach relation when subject is despawned
- Tracing
- Clear entity
- EntityBuilder hierarchy
- User guide
- Query
- Schedule
- Filter for &Filter
- Relation and wildcard for `with` and `without`
- Make storage self contained
- Batch insert
- Column serialization and deserialization
- Row and column serialization
- Relations_like
- Entity builder and batch spawn
- Cmd batch
- Hierarchy
- Commandbuffer
- FetchItem
- Allow filters to be attached directly to a fetch
- Merge worlds
- Merge custom components and relations
- Fast path on extend for empty archetype
- On_removed channel
- Shared system resource
- Use normal references in systems
- Allow schedle introspection
- Merge change ticks
- Auto opt in test
- Feature gate implementation detail asserts
- Serialization

### Bug Fixes

- Wip issues
- PreparedQuery re-entrancy
- Spawn_at
- Empty entities in root archetype
- Guide workflow
- Guide workflow
- Assertion not respecting groups
- Non sorted change list
- Release assertion on non unqiue fn instances
- Id recycling
- Update markdown title
- Docs and unnused items
- Dead code
- ComponentBuffer non deterministic iteration order
- Clippy lints
- Cursor position outside buffer
- Vastly simplify system traits
- Docs and warnings
- Don't expose rexport buffer
- Inconsistent Fetch trait
- Bincode serialization
- On_remove not triggered for clear
- Merge with holes in entity ids
- Commandbuffer events not happening in order
- Query not recalculating archetypes when entity moves to existing but empty arch
- Change event wrapping
- Warnings
- SystemFn describe
- Use of unstable features
- Imports and serde api
- QueryBorrow::get
- Broken link
- Miri
- Badge style
- Make queries skip empty archetypes in access
- Sync readme
- Execute schedule in doc test
- Test with all features
- Wrapped line in docs
- Hide extraneous bracket
- Docs
- Stable archetype gen
- Unused deps
- Public api
- Cleanup public api
- Continue api cleanup
- Link style
- Missing import
- Broken doclinks
- Derive docs
- Manifest
- Bump deps
- Eprintln

### Documentation

- Relations

### Refactor

- Simplify filter
- Archetype storage
- Entity spawning
- Change list
- Shared resource

### Miscellaneous Tasks

- Remove dbg prints
- Fix all warnings
- Apply clippy lints
- Add guide to readme
- More comments in examples
- Sync readme
- More links
- Small changes
- Reduce items in prelude
- Change default query generics
- Custom EntityKind [de]serialize implementation
- Sync readme
- Link relations in docs
- Sync readme
- Bump version

### Update

- Workflows

<!-- generated by git-cliff -->
