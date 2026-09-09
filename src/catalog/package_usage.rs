use super::wally_packages::{self, PackageSpec, Realm};

pub struct Guide {
    pub when: &'static str,
    pub context: &'static str,
    pub imports: &'static [&'static str],
    pub example: &'static str,
    pub caveats: &'static str,
}

pub fn find(key: &str) -> Option<Guide> {
    let (when, context, imports, example, caveats): (_, _, &[&str], _, _) = match key {
        "react" => (
            "Describe reusable UI components with props and hooks.",
            "Client ModuleScript; return a component for your ReactRoblox root.",
            &["react"],
            "return function()\n    return React.createElement(\"TextLabel\", {\n        Text = \"Ready\",\n        Size = UDim2.fromOffset(160, 40),\n    })\nend",
            "An element is a description, not an Instance. Mount it with ReactRoblox; call hooks only inside components.",
        ),
        "reactRoblox" => (
            "Mount React components into Roblox Instances.",
            "Client LocalScript; select both react and reactRoblox.",
            &["react", "reactRoblox"],
            "local gui = Instance.new(\"ScreenGui\")\ngui.Parent = game:GetService(\"Players\").LocalPlayer.PlayerGui\nlocal root = ReactRoblox.createRoot(gui)\nroot:render(React.createElement(\"TextLabel\", {\n    Text = \"Ready\", Size = UDim2.fromOffset(160, 40),\n}))",
            "Call root:unmount() and gui:Destroy() when this UI is no longer needed.",
        ),
        "vide" => (
            "Build reactive UI without a virtual element tree.",
            "Client LocalScript.",
            &["vide"],
            "local dispose = Vide.root(function()\n    local count = Vide.source(0)\n    Vide.effect(function() print(count()) end)\n    count(1)\nend)\ndispose()",
            "Keep effects and UI inside a root; dispose the root when the screen is removed.",
        ),
        "fusion" => (
            "Model reactive UI state with explicit lifetime scopes.",
            "Client LocalScript.",
            &["fusion"],
            "local scope = Fusion.scoped(Fusion)\nlocal count = scope:Value(0)\ncount:set(1)\nprint(Fusion.peek(count))\nFusion.doCleanup(scope)",
            "This is Fusion 0.3's scoped API, not older 0.2 examples.",
        ),
        "matter" => (
            "Organize gameplay as entities, components, and systems.",
            "Server Script or shared ModuleScript.",
            &["matter"],
            "local Health = Matter.component(\"Health\")\nlocal world = Matter.World.new()\nlocal entity = world:spawn(Health({ value = 100 }))\nprint(world:get(entity, Health).value)\nworld:despawn(entity)",
            "This creates a local world; replication and scheduling systems are separate decisions.",
        ),
        "reflex" => (
            "Centralize immutable state updates and selectors.",
            "Shared ModuleScript; each runtime has independent state.",
            &["reflex"],
            "local producer = Reflex.createProducer({ coins = 0 }, {\n    addCoins = function(state, amount)\n        return { coins = state.coins + amount }\n    end,\n})\nproducer.addCoins(5)\nprint(producer:getState().coins)",
            "Do not mutate the old state inside an action. Replication requires additional setup.",
        ),
        "reactReflex" => (
            "Render Reflex state through React components.",
            "Client component ModuleScript; place beneath ReflexProvider.",
            &["react", "reactReflex"],
            "return function()\n    local coins = ReactReflex.useSelector(function(state)\n        return state.coins\n    end)\n    return React.createElement(\"TextLabel\", {\n        Text = tostring(coins), Size = UDim2.fromOffset(160, 40),\n    })\nend",
            "Select reflex too. Wrap the component in ReactReflex.ReflexProvider with producer = yourProducer.",
        ),
        "charm" => (
            "Share small reactive state without a central reducer.",
            "Shared ModuleScript; server and client state are separate.",
            &["charm"],
            "local count = Charm.atom(0)\nlocal stop = Charm.effect(function() print(count()) end)\ncount(1)\nstop()",
            "Use immutable updates for tables. Dispose effects; sharing a module does not replicate its state.",
        ),
        "charmSync" => (
            "Connect Charm state to your client/server transport.",
            "Client setup snippet; select charm too.",
            &["charm", "charmSync"],
            "local coins = Charm.atom(0)\nCharmSync.client.addSignals({ coins = coins })\n-- Later, during teardown:\nCharmSync.client.removeSignals(\"coins\")",
            "Registration alone sends nothing. Wire server.addSignalsToClient, server.connect, and client.sync through your remotes; validate client requests server-side.",
        ),
        "reactCharm" => (
            "Re-render a React component when Charm state changes.",
            "Client component ModuleScript; select react and charm too.",
            &["react", "charm", "reactCharm"],
            "local coins = Charm.atom(0)\nreturn function()\n    local value = ReactCharm.useSignalState(coins)\n    return React.createElement(\"TextLabel\", {\n        Text = tostring(value), Size = UDim2.fromOffset(160, 40),\n    })\nend",
            "Call this hook inside a mounted React component; expose the atom separately when other modules need to update it.",
        ),
        "videCharm" => (
            "Expose Charm state as a Vide source.",
            "Client LocalScript; select vide and charm too.",
            &["vide", "charm", "videCharm"],
            "local coins = Charm.atom(0)\nlocal dispose = Vide.root(function()\n    local value = VideCharm.useSignalState(coins)\n    Vide.effect(function() print(value()) end)\n    coins(5)\nend)\ndispose()",
            "Create the binding inside a Vide root so its subscription is cleaned up.",
        ),
        "lyra" => (
            "Maintain versioned player data and migration steps.",
            "Server ModuleScript; a migration declaration only.",
            &["lyra"],
            "local addSettings = Lyra.MigrationStep.addFields(\n    \"add-settings\", { Music = true }\n)\nreturn addSettings",
            "A migration does not open a store. Follow createPlayerStore configuration and lifecycle documentation before using real player data.",
        ),
        "profilestore" => (
            "Persist player profiles with session locking.",
            "Server Script only.",
            &["profilestore"],
            "local store = ProfileStore.New(\"ExampleProfiles\", { Coins = 0 })\nprint(store.Name)",
            "Store creation is not a complete player lifecycle. Handle StartSessionAsync failure, reconciliation, PlayerRemoving, and EndSession. Never expose profile data through shared submodules unintentionally.",
        ),
        "scribe" => (
            "Declare typed, replicated profile schemas.",
            "Shared ModuleScript; a schema declaration only.",
            &["scribe"],
            "return {\n    Coins = Scribe.Number(0),\n    Spawn = Scribe.Vector3(Vector3.zero),\n}",
            "Pass the schema as Template to Scribe.new with your ProfileStoreIndex and ProfileKeyPrefix. Typed accessors need the new Luau type solver; follow the full server lifecycle before storing real data.",
        ),
        "testez" => (
            "Add portable BDD-style unit specs.",
            "tests/shared/example.spec.luau ModuleScript; Testing = TestEZ.",
            &[],
            "return function()\n    describe(\"addition\", function()\n        it(\"adds two numbers\", function()\n            expect(2 + 3).to.equal(5)\n        end)\n    end)\nend",
            "Run rproj test from the project directory. These globals are supplied by TestEZ, not by Jest.",
        ),
        "jest" | "jest-globals" => (
            "Run explicit-import specs in Roblox Studio or Open Cloud CI.",
            "tests/shared/example.spec.luau ModuleScript; Testing = Jest Roblox.",
            &["jest-globals"],
            "JestGlobals.describe(\"addition\", function()\n    JestGlobals.it(\"adds two numbers\", function()\n        JestGlobals.expect(2 + 3).toBe(5)\n    end)\nend",
            "Wally only. Run rproj test; local execution needs Studio and the Jest runner plugin. DevPackages and tests exist only in jest.project.json, not the production project.",
        ),
        "janitor" => (
            "Collect connections and Instances for deterministic cleanup.",
            "Client or server Script.",
            &["janitor"],
            "local cleanup = Janitor.new()\nlocal part = Instance.new(\"Part\")\ncleanup:Add(part, \"Destroy\")\ncleanup:Destroy()",
            "Use one owner per resource lifetime; clean up when that owner is destroyed.",
        ),
        "ripple" => (
            "Animate numbers and Roblox value types.",
            "Client or server Script; manually stepped example.",
            &["ripple"],
            "local motion = Ripple.createMotion(0, { start = false })\nmotion:spring(1)\nprint(motion:step(1 / 60))\nmotion:destroy()",
            "For continuous animation, start the motion or step it each frame; stop/destroy it on teardown.",
        ),
        "reactRipple" => (
            "Bind Ripple animations to React properties.",
            "Client component ModuleScript; select react and ripple too.",
            &["react", "reactRipple"],
            "return function()\n    local transparency, motion = ReactRipple.useMotion(1)\n    React.useEffect(function() motion:spring(0) end, {})\n    return React.createElement(\"Frame\", {\n        BackgroundTransparency = transparency,\n        Size = UDim2.fromOffset(160, 40),\n    })\nend",
            "Hooks require a mounted component. useMotion owns the motion subscription's cleanup.",
        ),
        "prettyReactHooks" => (
            "Reuse common React event, timing, and viewport hooks.",
            "Client component ModuleScript; select react too.",
            &["react", "prettyReactHooks"],
            "return function()\n    local viewport = PrettyReactHooks.useViewport()\n    return React.createElement(\"TextLabel\", {\n        Text = viewport:map(function(size) return tostring(size) end),\n        Size = UDim2.fromOffset(200, 40),\n    })\nend",
            "useViewport returns a React binding, not a plain Vector2. Mount with ReactRoblox.",
        ),
        "videRipple" => (
            "Use Ripple animations as Vide sources.",
            "Client LocalScript; select vide and ripple too.",
            &["vide", "videRipple"],
            "local dispose = Vide.root(function()\n    local value, motion = VideRipple.useMotion(0)\n    motion:spring(1)\n    Vide.effect(function() print(value()) end)\nend)\ntask.delay(1, dispose)",
            "The animation lives inside the Vide root and stops when that root is disposed.",
        ),
        "remo" => (
            "Declare typed remotes in a shared module.",
            "Shared ModuleScript required by both server and client.",
            &["remo"],
            "return Remo.createRemotes({\n    ping = Remo.remote(),\n})",
            "Declarations are not authorization. Validate payloads, permissions, and request rates on the server before acting on client input.",
        ),
        "promise" => (
            "Compose asynchronous work and handle rejection.",
            "Client or server Script.",
            &["promise"],
            "Promise.resolve(5)\n    :andThen(function(value) print(value * 2) end)\n    :catch(function(message) warn(message) end)",
            "Handle rejections. Cancel owned asynchronous work when its caller is destroyed.",
        ),
        "greentea" => (
            "Validate runtime values with typed schemas.",
            "Shared ModuleScript or server boundary validation.",
            &["greentea"],
            "local nameType = gt.build(gt.string())\nnameType:assert(\"Player\")",
            "Assertions throw on invalid data. Handle untrusted input deliberately; a valid type does not imply permission.",
        ),
        "t" => (
            "Validate values at runtime using composable predicates.",
            "Shared ModuleScript or server boundary validation.",
            &["t"],
            "local isScore = t.interface({ score = t.number })\nlocal ok, message = isScore({ score = 10 })\nassert(ok, message)",
            "Validation checks shape, not authorization or rate limits.",
        ),
        "sift" => (
            "Make immutable array and dictionary transformations.",
            "Shared ModuleScript.",
            &["sift"],
            "local original = { coins = 0 }\nlocal updated = Sift.Dictionary.merge(original, { coins = 5 })\nprint(original.coins, updated.coins)",
            "The catalog records this as community-stable, not actively developed. Check nested-copy semantics before modifying nested tables.",
        ),
        _ => return None,
    };
    Some(Guide {
        when,
        context,
        imports,
        example,
        caveats,
    })
}

pub fn import(package: &PackageSpec, submodules: bool) -> String {
    let path = if submodules {
        format!(
            "game:GetService(\"ReplicatedStorage\").modules.{}",
            package.module_name
        )
    } else {
        let (service, mount) = match package.realm {
            Realm::Shared => ("ReplicatedStorage", "packages"),
            Realm::Server => ("ServerScriptService", "serverPackages"),
            Realm::Dev => ("ReplicatedStorage", "DevPackages"),
        };
        format!("game:GetService(\"{service}\").{mount}.{}", package.alias())
    };
    format!("local {} = require({path})", package.module_name)
}

pub fn example(guide: &Guide) -> String {
    let mut lines: Vec<_> = guide
        .imports
        .iter()
        .map(|key| {
            import(
                wally_packages::find(key).expect("guide import exists"),
                false,
            )
        })
        .collect();
    lines.push(guide.example.into());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_existing_package_has_an_example_with_resolved_imports() {
        for package in wally_packages::PACKAGES {
            let guide = find(package.key).expect(package.key);
            assert!(!guide.example.is_empty());
            assert!(!guide.when.is_empty());
            for key in guide.imports {
                assert!(wally_packages::find(key).is_some(), "{key}");
            }
            assert!(!example(&guide).is_empty());
        }
        assert!(
            import(wally_packages::find("profilestore").unwrap(), false)
                .contains("ServerScriptService\").serverPackages.profilestore")
        );
        assert!(
            import(wally_packages::find("jest-globals").unwrap(), false)
                .contains("DevPackages.JestGlobals")
        );
        assert!(import(wally_packages::find("charm").unwrap(), true).contains("modules.Charm"));
    }
}
