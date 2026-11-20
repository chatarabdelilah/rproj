"""Live, disposable acceptance for the matching built worker (Windows, Python 3.11+)."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib


def run(cwd, env, *args, check=True):
    result = subprocess.run(args, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                            capture_output=True, text=True, encoding='utf-8',
                            errors='replace', timeout=300)
    if check and result.returncode:
        raise RuntimeError(f'{args[0]} exited {result.returncode}: {result.stderr[-1500:]}')
    return result


def prove(root, binary, target):
    project = root / 'Test démo'
    project.mkdir()
    env = {**os.environ}
    if target == 'rokit':
        env['ROKIT_ROOT'] = str(root / 'rokit-storage')
        Path(env['ROKIT_ROOT']).mkdir()
        run(root, env, 'rokit', '--version')
        run(project, env, 'rokit', 'trust', 'rojo-rbx/rojo')
        record = project / 'rokit.toml'
        record.write_text('# preserve comment\n[tools]\neditor = "rojo-rbx/rojo@7.6.1"\n')
    else:
        record = project / 'wally.toml'
        record.write_text('[package]\nname = "proof/live"\nversion = "0.1.0"\nrealm = "shared"\n'
                          'registry = "https://github.com/UpliftGames/wally-index"\n'
                          '[dependencies]\nSignals = "sleitnick/signal@=1.5.0"\n')
        (project / 'default.project.json').write_text(json.dumps({'name': 'Live', 'tree': {
            '$className': 'DataModel', 'ReplicatedStorage': {'Packages': {'$path': 'Packages'}}}}))
        run(project, env, 'wally', 'install')
        run(project, env, 'rojo', 'sourcemap', 'default.project.json', '--output', 'sourcemap.json')
        run(project, env, 'wally-package-types', '-s', 'sourcemap.json', 'Packages')

    observed = json.loads(run(project, env, binary, 'updates', '--json').stdout)
    entry = next(e for e in observed['entries'] if e['local']['alias'] in ('editor', 'Signals'))
    version = entry['comparison']['latest_stable']
    if not entry['observed'] or not version:
        raise RuntimeError('Public release observation unavailable')
    selection = {'project': observed['project'], 'local': entry['local'], 'version': version}

    def job(action, expect_success=True):
        with tempfile.TemporaryDirectory(dir=root) as path:
            ipc = Path(path)
            req = {'id': ipc.name, 'protocol': 1, 'action': action}
            (ipc / 'request.json').write_text(json.dumps(req),encoding='utf-8')
            result = run(project, env, binary, '--rproj-workbench', 'reviewed-update-job', path, check=False)
            report = json.loads((ipc / 'report.json').read_text(encoding='utf-8'))
            if report['id'] != ipc.name or (result.returncode == 0) != expect_success:
                raise RuntimeError(f'Unexpected worker outcome: {report["state"]}')
            return report['state']

    original = record.read_bytes()
    if target == 'wally':
        extra = project / 'Packages' / 'local.luau'
        extra.write_text('-- preserve')
        blocked = job({'Prepare': selection})['Prepared']
        if not any('local.luau' in b for b in blocked['blockers']):
            raise RuntimeError('Local file did not block Apply')
        if job({'Apply': blocked}, False)['Failed']['possible_writes'] or extra.read_text() != '-- preserve':
            raise RuntimeError('Blocked Apply disturbed the project')
        extra.unlink()
    plan = job({'Prepare': selection})['Prepared']
    if plan['blockers'] or record.read_bytes() != original:
        raise RuntimeError(f'Unexpected preparation: {plan["blockers"]}')
    record.write_bytes(original + b'# external change\n')
    refusal = job({'Apply': plan}, False)['Failed']
    if refusal['possible_writes'] or 'changed' not in refusal['reason']:
        raise RuntimeError('Stale Apply reported writes')
    plan = job({'Prepare': selection})['Prepared']
    if target == 'rokit':
        cache = root / 'warm-cache'
        cache.mkdir()
        (cache / 'rokit.toml').write_text('[tools]\n')
        run(cache, env, 'rokit', 'add', f'rojo-rbx/rojo@{version}', 'editor')
        artifact = Path(env['ROKIT_ROOT']) / 'tool-storage/rojo-rbx/rojo' / version / 'rojo.exe'
        released = artifact.read_bytes()
        artifact.write_bytes(b'incorrect cached version')
        refusal = job({'Apply': plan}, False)['Failed']
        if 'Cached executable differs' not in refusal['reason'] or not refusal['possible_writes']:
            raise RuntimeError(f'Expected explicit cached artifact refusal: {refusal}')
        if tomllib.loads(record.read_text())['tools']['editor'] != 'rojo-rbx/rojo@7.6.1':
            raise RuntimeError('Corrupt cache changed the project pin')
        artifact.write_bytes(released)
        plan = job({'Prepare': selection})['Prepared']
    if job({'Apply': plan}) != 'Applied':
        raise RuntimeError('Missing confirmed success')
    key, alias = ('tools', 'editor') if target == 'rokit' else ('dependencies', 'Signals')
    if (not tomllib.loads(record.read_text())[key][alias].endswith(version)
            or b'# external change' not in record.read_bytes()):
        raise RuntimeError('Final declaration differs from review')
    print(f'{target}: stale review refused, confirmed {alias} {version} in a disposable project.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('target', choices=['wally', 'rokit'])
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='rproj-worker-proof-') as directory:
        prove(Path(directory), args.binary.resolve(strict=True), args.target)
