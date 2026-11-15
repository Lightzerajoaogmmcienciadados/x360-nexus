#!/usr/bin/env python3
"""Package the Pinyon Shift 0.8.2 Android *shell* from its official ShiftGlue kit.

This does NOT compile, include or distribute any Forza Horizon game code/assets.
The resulting APK contains only the native runtime and the Java setup UI.
The game-specific libmain.so and other generated game binaries are absent.
"""

import argparse
import io
import json
from pathlib import Path
import tarfile
import zipfile

PREFIX = 'shiftglue-kit/'
LIBS = (
    'libfile_redirect_hook.so',
    'libgsl_alloc_hook.so',
    'libhook_impl.so',
    'libmain_hook.so',
    'librexgpu-fh1.so',
    'librexruntime.so',
)


def extract_required(tar, name):
    try:
        member = tar.getmember(PREFIX + name)
    except KeyError as e:
        raise SystemExit(f'Missing {PREFIX + name} in kit: {e}')
    f = tar.extractfile(member)
    if not f:
        raise SystemExit(f'Not a regular file: {member.name}')
    return f.read()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--kit', type=Path, required=True, help='android-shell-*.tar.xz file')
    p.add_argument('--libcxx', type=Path, required=True, help='libc++_shared.so from Android NDK')
    p.add_argument('--output', type=Path, required=True, help='unsigned APK output')
    args = p.parse_args()

    if not args.libcxx.is_file() or args.libcxx.stat().st_size < 100_000:
        raise SystemExit('NDK libc++_shared.so missing or unexpectedly small')
    if args.libcxx.open('rb').read(4) != b'\x7fELF':
        raise SystemExit('NDK libc++_shared.so does not have an ELF header')

    with tarfile.open(args.kit, 'r:xz') as tar:
        kit_json = json.loads(extract_required(tar, 'kit.json'))
        if kit_json.get('platform') != 'android-arm64':
            raise SystemExit('This script expects an android-arm64 kit')
        base = extract_required(tar, 'android/base.apk')
        dex = extract_required(tar, 'android/classes.dex')
        if not dex.startswith(b'dex\n'):
            raise SystemExit('Invalid classes.dex in shell')

        args.output.parent.mkdir(parents=True, exist_ok=True)
        with (zipfile.ZipFile(io.BytesIO(base), 'r') as bas,
              zipfile.ZipFile(args.output, 'w', compression=zipfile.ZIP_DEFLATED,
                              compresslevel=7, allowZip64=True) as apk):
            for entry in bas.infolist():
                if entry.is_dir() or entry.filename.startswith('META-INF/'):
                    continue
                apk.writestr(entry.filename, bas.read(entry.filename), compress_type=zipfile.ZIP_DEFLATED)
            apk.writestr('classes.dex', dex, compress_type=zipfile.ZIP_DEFLATED)
            for name in LIBS:
                data = extract_required(tar, f'lib/{name}')
                if not data.startswith(b'\x7fELF'):
                    raise SystemExit(f'Invalid ELF: {name}')
                apk.writestr(f'lib/arm64-v8a/{name}', data,
                             compress_type=zipfile.ZIP_DEFLATED)
                print(f'Added {name}: {len(data):,} bytes')
            apk.write(args.libcxx, 'lib/arm64-v8a/libc++_shared.so',
                      compress_type=zipfile.ZIP_DEFLATED)
    print(f'Created shell-only unsigned APK: {args.output} ({args.output.stat().st_size:,} bytes)')
    print('WARNING: The kit has no game-specific libmain.so or game assets. This is NOT a playable game APK.')


if __name__ == '__main__':
    main()
