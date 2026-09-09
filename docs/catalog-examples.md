# Catalog Example Evidence

T2 bundles guidance for every existing package in `src/catalog/package_usage.rs`.
Both named plain lookups and the TUI render that data through `catalog_view`.
Examples are authored for rproj's generated mounts, not copied with upstream
sample aliases that differ from the manifest.

## Version Review

September 8-9, 2026: inspected the exact published package archives from the
[official Wally registry API](https://api.wally.run/v1/package-contents/littensy/charm/0.11.0).
Archive URLs have the shape `/v1/package-contents/<scope>/<name>/<version>`;
requests use the `Wally-Version: 0.3.2` header. Local evidence is extracted under
`target/catalog-evidence`, not shipped in the crate. Repository documentation
links remain in each package's existing catalog entry.

| Catalog key(s) and reviewed version | API checked |
| --- | --- |
| react, reactRoblox 17.2.1 | createElement, ReactRoblox createRoot/render/unmount; source exports |
| vide 0.4.1 | root returns disposer; source/effect exports |
| fusion 0.3.0 | scoped, Value, peek, doCleanup exports |
| matter 0.8.4 | component, World.new/spawn/get/despawn |
| reflex 4.3.1 | createProducer, action dispatch, getState; producer has no destroy method |
| reactReflex 0.3.6 | useSelector and ReflexProvider producer property |
| charm 0.11.0 | atom and effect disposer; this archive also exports signal APIs |
| charmSync 0.4.0 | client.addSignals/removeSignals; server transport is separate |
| reactCharm, videCharm 0.4.0 | useSignalState and lifecycle cleanup |
| lyra 0.6.0 | MigrationStep.addFields(name, fields) |
| profilestore 1.0.3 | ProfileStore.New and store.Name; no sessions opened by the example |
| scribe 2.2.0 | Number/Vector3 schema constructors; template/store prerequisites |
| testez 0.4.1 | describe/it/expect convention supplied inside a returned spec function |
| jest, jest-globals 3.20.1 | explicit JestGlobals imports; describe/it/expect.toBe |
| janitor 1.18.3 | new/Add/Destroy |
| ripple 0.10.2 | createMotion, spring, manual step, destroy |
| reactRipple 3.0.1 | useMotion returns binding and motion; effect cleanup |
| prettyReactHooks 0.1.1 | useViewport returns Binding<Vector2>, not Vector2 |
| videRipple 0.10.2 | useMotion returns a source and motion within a Vide root |
| remo 1.5.3 | createRemotes and remote builder; validation/authorization remain caller-owned |
| promise 4.0.0 | resolve/andThen/catch |
| greentea 0.4.11 | build(string()) and checker:assert |
| t 3.1.1 | interface and number predicate |
| sift 0.0.11 | Dictionary.merge returns a new dictionary |

## Mounts and Runtime Coverage

Wally imports derive from the same `PackageSpec::alias` as the manifest writer:
`ReplicatedStorage.packages.<key>`, `ServerScriptService.serverPackages.<key>`,
or `ReplicatedStorage.DevPackages.Jest/JestGlobals`. Submodule alternatives use
`ReplicatedStorage.modules.<module_name>`, and appear only when that dependency
closure is vendorable. Submodules track actual Git commits, not Wally versions;
their API must be checked independently. No workflow or dependency pin changes
are made by opening the Catalog.

The ignored real-Jest stack test installs the exact catalogued Wally packages,
regenerates/retypes its sourcemap, and executes the bundled Charm, Reflex, Matter,
Sift, t, GreenTea, Janitor, and Ripple snippets in Studio. It checks that all eight
execute without runtime errors and that Jest reports zero failures. It does not
prove every semantic claim made by those upstream libraries.

React mounting/hooks, Vide/Fusion lifecycles, replication transports, real data
stores, theme activation, and every submodule checkout are not runtime-certified
by these examples. Data examples deliberately stop at schema/store declarations;
the Catalog points readers to upstream lifecycle/security guidance. Use the
release audit for current executions rather than treating test existence as a pass.
