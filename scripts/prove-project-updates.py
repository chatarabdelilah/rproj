"""Exercise released tools in fresh projects. Requires Python 3.11+ and network."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib


def run(cwd, *args, env=None):
    result = subprocess.run(args, cwd=cwd, env=env, check=True, timeout=240,
                            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            text=True, encoding="utf-8", errors="replace")
    return result.stdout.strip()


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def manifest(requirement, second_alias=False):
    extra = ('[dev-dependencies]\nTestKnit = "sleitnick/knit@^1.5.0"\n'
             if second_alias else '')
    return ('[package]\nname = "proof/workspace"\nversion = "0.1.0"\n'
            'registry = "https://github.com/UpliftGames/wally-index"\nrealm = "shared"\n'
            '[dependencies]\nKnit = "sleitnick/knit@' + requirement + '"\n'
            'Promise = "evaera/promise@=4.0.0"\n' + extra)


def graph(project):
    packages = tomllib.loads((project / 'wally.lock').read_text())['package']
    root = next(package for package in packages if package['name'] == 'proof/workspace')
    return dict(root['dependencies']), sorted((p['name'], p['version']) for p in packages)


def installed_files(project):
    files = {}
    for folder in ('Packages', 'ServerPackages', 'DevPackages'):
        base = project / folder
        for file in sorted(base.rglob('*')):
            require(not file.is_symlink() and not getattr(file, 'is_junction', lambda: False)(),
                    f'Linked fixture path: {file}')
            if file.is_file():
                files[file.relative_to(project)] = hashlib.sha256(file.read_bytes()).hexdigest()
    return files


def prove_wally(root, wally, rojo, types):
    require(run(root, wally, '--version') == 'wally 0.3.2', 'Expected Wally 0.3.2')

    def install(name, requirement, lock=None, alias=False):
        project = root / name
        project.mkdir()
        (project / 'wally.toml').write_text(manifest(requirement, alias), encoding='utf-8')
        if lock:
            shutil.copyfile(lock / 'wally.lock', project / 'wally.lock')
        run(project, wally, 'install')
        dirs = [d for d in ('Packages', 'ServerPackages', 'DevPackages') if (project / d).exists()]
        tree = {'$className': 'DataModel', 'ReplicatedStorage': {d: {'$path': d} for d in dirs}}
        (project / 'default.project.json').write_text(json.dumps({'name': 'Proof', 'tree': tree}))
        run(project, rojo, 'sourcemap', 'default.project.json', '--output', 'sourcemap.json')
        run(project, types, '-s', 'sourcemap.json', *dirs)
        return project

    old = install('Original', '=1.5.3')
    baseline = install('Baseline replay', '^1.5.0', old)
    require(graph(old) == graph(baseline), 'Existing lock changed during replay')
    expected_files = installed_files(baseline)
    require(expected_files and installed_files(old) == expected_files, 'Baseline output differs')
    (old / 'Packages' / 'Knit.lua').write_text('-- local edit')
    (old / 'Packages' / 'custom.luau').write_text('-- unrecorded')
    observed = installed_files(old)
    differences = {p for p in observed.keys() | expected_files.keys()
                   if observed.get(p) != expected_files.get(p)}
    require(differences == {Path('Packages/Knit.lua'), Path('Packages/custom.luau')},
            f'Could not identify both local files: {differences}')
    prepared = install('Pinned target', '=1.6.0', baseline)
    for i, requirement in enumerate(('^1.5.0', '=1.6.0', '^1.6.0')):
        replay = install(f'Replay {i}', requirement, prepared)
        require((replay / 'wally.lock').read_bytes() == (prepared / 'wally.lock').read_bytes()
                and installed_files(replay) == installed_files(prepared), 'Prepared replay differs')
    before, old_nodes = graph(baseline)
    after, new_nodes = graph(prepared)
    require(after == {**before, 'Knit': 'sleitnick/knit@1.6.0'}, 'Unexpected direct change')
    require([p for p in old_nodes if p[0] == 'sleitnick/comm']
            != [p for p in new_nodes if p[0] == 'sleitnick/comm'], 'Missing transitive change')
    aliases = install('Two aliases', '=1.5.3', alias=True)
    candidate = install('Changed aliases', '=1.6.0', aliases, alias=True)
    require(graph(aliases)[0]['TestKnit'] != graph(candidate)[0]['TestKnit'],
            'Expected collateral change to dev-dependencies.TestKnit')
    print('Wally passed: lock and typed output replay, direct aliases, transitive changes, local edits.')


def prove_rokit(root, rokit):
    store = root / 'rokit-storage'
    env = {**os.environ, 'ROKIT_ROOT': str(store)}
    require(run(root, rokit, '--version', env=env) == 'rokit 1.2.0', 'Expected Rokit 1.2.0')
    project, staging = root / 'Project', root / 'Isolated install'
    project.mkdir()
    staging.mkdir()
    original = '# project comment\n[tools]\neditor = "rojo-rbx/rojo@7.7.0"\nfmt = "JohnnyMorganz/StyLua@2.5.2"\n'
    record = project / 'rokit.toml'
    record.write_bytes(original.encode())
    (staging / 'rokit.toml').write_text('[tools]\n')
    # Explicit trust is only for this disposable storage and this fixture.
    run(staging, rokit, 'trust', 'rojo-rbx/rojo', env=env)
    run(staging, rokit, 'add', 'rojo-rbx/rojo@7.7.1', 'editor', env=env)
    installed = tomllib.loads((staging / 'rokit.toml').read_text())['tools']
    require(installed == {'editor': 'rojo-rbx/rojo@7.7.1'}, 'Unexpected installed pin or alias')
    artifacts = list((store / 'tool-storage/rojo-rbx/rojo/7.7.1').glob('rojo*'))
    require(len(artifacts) == 1 and run(staging, artifacts[0], '--version', env=env) == 'Rojo 7.7.1',
            'Installed Rojo artifact could not be verified')
    require(record.read_bytes() == original.encode(), 'Install changed original project')
    reviewed = original.replace('rojo@7.7.0', 'rojo@7.7.1').encode()
    record.write_bytes(reviewed)
    require(tomllib.loads(record.read_text())['tools'] == {**installed, 'fmt': 'JohnnyMorganz/StyLua@2.5.2'},
            'Unrelated pin changed')
    print('Rokit passed: exact artifact, custom alias, unrelated pin and project comment preserved.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('target', choices=['wally', 'rokit'])
    for name in ('wally', 'rojo', 'wally-package-types', 'rokit'):
        parser.add_argument('--' + name, default=name)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='rproj reviewed update ') as path:
        if args.target == 'wally':
            prove_wally(Path(path), args.wally, args.rojo, args.wally_package_types)
        else:
            prove_rokit(Path(path), args.rokit)
