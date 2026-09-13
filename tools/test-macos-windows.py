#!/usr/bin/env python3
"""Build a selectable macOS test app and run only disposable-window acceptance."""
import argparse
import json
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--performance', action='store_true')
    parser.add_argument('--release', action='store_true')
    args = parser.parse_args()
    if sys.platform != 'darwin':
        parser.error('This probe requires macOS and an interactive desktop')
    root = Path(__file__).resolve().parents[1]
    built = subprocess.run(['cargo', 'test', '--locked', '--lib', '--no-run',
                            '--message-format=json'] + (['--release'] if args.release else []), cwd=root, stdout=subprocess.PIPE, text=True)
    executable = None
    for line in built.stdout.splitlines():
        message = json.loads(line)
        if message.get('reason') == 'compiler-message':
            print(message['message'].get('rendered', ''), file=sys.stderr, end='')
        if (message.get('reason') == 'compiler-artifact'
                and message.get('target', {}).get('name') == 'keysteer'
                and message.get('profile', {}).get('test')
                and message.get('executable')):
            executable = Path(message['executable'])
    if built.returncode:
        return built.returncode
    if executable is None:
        raise RuntimeError('Cargo did not report the library test executable')
    output = root / 'target' / 'native-window-tests'
    app = output / 'KeySteer Native Tests.app'
    contents = app / 'Contents'
    binary = contents / 'MacOS' / 'KeySteerNativeTests'
    binary.parent.mkdir(parents=True, exist_ok=True)
    info = plistlib.dumps(dict(CFBundleIdentifier='org.keysteer.native-window-tests',
        CFBundleName='KeySteer Native Tests', CFBundleDisplayName='KeySteer Native Tests',
        CFBundleExecutable=binary.name, CFBundlePackageType='APPL', CFBundleVersion='1',
        LSUIElement=True, LSEnvironment=dict(KEYSTEER_PROBE_REQUEST_ACCESSIBILITY='1')))
    plist = contents / 'Info.plist'
    # Keep the responsible application's signed executable stable across Rust
    # rebuilds. The child test binary lives beside the bundle, outside its seal.
    child = output / 'window-tests'
    shutil.copy2(executable, child)
    launcher = root / 'tests' / 'fixtures' / 'macos-test-launcher.m'
    import hashlib
    source_hash = hashlib.sha256(launcher.read_bytes()).hexdigest()
    stamp = output / 'launcher.sha256'
    if not (binary.exists() and plist.exists() and plist.read_bytes() == info
            and stamp.exists() and stamp.read_text() == source_hash):
        subprocess.run(['/usr/bin/clang', str(launcher), '-fobjc-arc', '-framework',
                        'Foundation', '-o', str(binary)], check=True)
        plist.write_bytes(info)
        subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(app)], check=True)
        stamp.write_text(source_hash)
    print(f'Test app: {app}', flush=True)
    if args.prepare_only:
        return 0
    log = output / 'result.log'
    log.write_text('')
    subprocess.run(['/usr/bin/open', '-n', '-W', '--stdout', str(log), '--stderr', str(log),
                    str(app), '--args', ('native_macos_tabs_geometry_performance' if args.performance else 'native_macos_window_parity'), '--ignored',
                    '--nocapture', '--test-threads=1'], check=True)
    result = log.read_text()
    print(result, end='')
    print(f'Log: {log}')
    return 0 if 'test result: ok. 1 passed;' in result else 1


if __name__ == '__main__':
    sys.exit(main())
