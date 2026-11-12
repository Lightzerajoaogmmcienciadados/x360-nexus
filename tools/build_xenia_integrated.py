#!/usr/bin/env python3
"""Fuse X360 Nexus Android shell + XenDroid Mali Xenia host into ONE APK.

NOT a native Rust port of the complete Xbox 360 emulator. This produces a
redistribution based on XenDroid Mali's original signed APK (q664), with an
additional X360 Nexus launcher and JNI/Rust diagnostics, preserving all
XenDroid resources and its JNI-bound native runtime.

The build workflow downloads the BSD-licensed upstream package, checks its
SHA-256, uses Apktool only for its binary Android manifest/resources, and signs
the derived artifact with a separate test key. See THIRD-PARTY-NOTICES.txt.
"""
import argparse
import re
import shutil
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

A = "{http://schemas.android.com/apk/res/android}"
ET.register_namespace("android", "http://schemas.android.com/apk/res/android")
ET.register_namespace("tools", "http://schemas.android.com/tools")


def dex_files(names):
    return sorted((name for name in names
                   if re.fullmatch(r"classes([2-9]|[1-9][0-9]+)?\.dex", name)),
                  key=lambda s: 1 if s == "classes.dex" else int(s[7:-4]))


def check_native(apk, required):
    with zipfile.ZipFile(apk) as z:
        names = set(z.namelist())
        for name in required:
            if name not in names:
                raise ValueError(f"{apk}: mandatory native component missing: {name}")
        return names


def integrate(upstream, nexus, decoded, license_file):
    upstream_names = check_native(upstream, [
        "lib/arm64-v8a/libe.so",
        "classes.dex",
        "AndroidManifest.xml",
    ])
    nexus_names = check_native(nexus, [
        "lib/arm64-v8a/libx360nexus.so",
        "classes.dex",
    ])
    if not (decoded / "AndroidManifest.xml").is_file():
        raise FileNotFoundError("Apktool did not decode AndroidManifest.xml")
    if not (decoded / "apktool.yml").is_file():
        raise FileNotFoundError("Apktool did not decode APK metadata")

    upstream_dex = dex_files(upstream_names)
    nexus_dex = dex_files(nexus_names)
    if not upstream_dex or not nexus_dex:
        raise ValueError("Both Android applications must have dex code")
    for name in upstream_dex:
        if not (decoded / name).is_file():
            raise ValueError(f"Apktool did not preserve {name}: use --no-src")

    with zipfile.ZipFile(nexus) as z:
        max_dex = max(1 if name == "classes.dex" else int(name[7:-4]) for name in upstream_dex)
        for index, name in enumerate(nexus_dex, start=max_dex + 1):
            target = decoded / f"classes{index}.dex"
            if target.exists():
                raise ValueError(f"DEX collision: {target}")
            with z.open(name) as data, target.open("wb") as output:
                shutil.copyfileobj(data, output)

        target_lib = decoded / "lib" / "arm64-v8a" / "libx360nexus.so"
        target_lib.parent.mkdir(parents=True, exist_ok=True)
        with z.open("lib/arm64-v8a/libx360nexus.so") as data, target_lib.open("wb") as output:
            shutil.copyfileobj(data, output)

        # Preserve a scaled version of the user-supplied previous X360 Nexus
        # screenshot in the combined APK, without changing its layout.
        photo = "assets/images/x360-nexus-anterior.jpg"
        if photo not in nexus_names:
            raise ValueError("Previous Nexus screenshot not found in app assets")
        destination = decoded / photo
        destination.parent.mkdir(parents=True, exist_ok=True)
        with z.open(photo) as image_data, destination.open("wb") as image_out:
            shutil.copyfileobj(image_data, image_out)

    doc = ET.parse(decoded / "AndroidManifest.xml")
    root = doc.getroot()
    package = root.get("package")
    if package != "xendroid.compose":
        raise ValueError(f"Unexpected XenDroid package {package!r}; refusing unsafe patch")
    application = root.find("application")
    if application is None:
        raise ValueError("No upstream application")

    # Keep the upstream Application and its emulation host process/services.
    base_app = application.get(A + "name", "")
    if "Application" not in base_app:
        raise ValueError(f"Unexpected XenDroid Application {base_app!r}")
    host = [activity for activity in application.findall("activity")
            if activity.get(A + "name", "").endswith("EmulatorHostActivity")]
    if len(host) != 1:
        raise ValueError("Original Xenia emulation host is missing or ambiguous")

    launcher_found = False
    for activity in application.findall("activity"):
        if activity.get(A + "name", "").endswith("MainActivity"):
            for filt in activity.findall("intent-filter"):
                for cat in list(filt.findall("category")):
                    if cat.get(A + "name") == "android.intent.category.LAUNCHER":
                        filt.remove(cat)
                        launcher_found = True
    if not launcher_found:
        raise ValueError("Could not locate the original launcher")
    if any(x.get(A + "name") == "com.x360nexus.MainActivity"
           for x in application.findall("activity")):
        raise ValueError("Nexus launcher already exists")

    root.set(A + "versionCode", "1004")
    root.set(A + "versionName", "10.4-xenia-mali-dual-core")
    application.set(A + "label", "X360 Nexus 10.4 • Xenia Mali")
    nexus_activity = ET.SubElement(application, "activity")
    nexus_activity.set(A + "name", "com.x360nexus.MainActivity")
    nexus_activity.set(A + "exported", "true")
    nexus_activity.set(A + "theme", "@android:style/Theme.Material.NoActionBar")
    nexus_activity.set(A + "configChanges", "orientation|screenSize|keyboardHidden")
    intent = ET.SubElement(nexus_activity, "intent-filter")
    action = ET.SubElement(intent, "action")
    action.set(A + "name", "android.intent.action.MAIN")
    category = ET.SubElement(intent, "category")
    category.set(A + "name", "android.intent.category.LAUNCHER")

    # XenDroid q664 EmulatorHostActivity requires this special access for
    # real-path emulation. The user must grant it in Android Settings.
    if not any(p.get(A + "name") == "android.permission.MANAGE_EXTERNAL_STORAGE"
               for p in root.findall("uses-permission")):
        perm=ET.SubElement(root,"uses-permission")
        perm.set(A + "name","android.permission.MANAGE_EXTERNAL_STORAGE")

    if not any(p.get(A + "name") == "android.permission.VIBRATE"
               for p in root.findall("uses-permission")):
        vib = ET.SubElement(root, "uses-permission")
        vib.set(A + "name", "android.permission.VIBRATE")

    doc.write(decoded / "AndroidManifest.xml", encoding="utf-8",
              xml_declaration=True)

    notices = decoded / "assets" / "X360_NEXUS_XENIA_ENGINE_NOTICES.txt"
    notices.parent.mkdir(parents=True, exist_ok=True)
    notices.write_text(
        "X360 NEXUS - THIRD-PARTY XBOX 360 ENGINE\n"
        "This APK integrates components from XenDroid Ultimate Mali q664, "
        "based on Xenia/Xenia Canary, and the upstream XenDroid Android port.\n"
        "Source code: https://github.com/DEGROUCHO/xendroid-mali\n"
        "Source code for X360 Nexus shell: "
        "https://github.com/Lightzerajoaogmmcienciadados/x360-nexus\n"
        "See the original sources for third-party dependencies and their "
        "individual licenses. This software is NOT affiliated with Microsoft "
        "or any game publisher.\n\n"
        + license_file.read_text(encoding="utf-8")
        + "\n",
        encoding="utf-8"
    )
    print(f"Original XenDroid dex: {len(upstream_dex)}")
    print(f"X360 Nexus dex appended: {len(nexus_dex)}")
    print(f"Xenia libe.so preserved: YES")
    print(f"X360 Nexus libx360nexus.so added: YES")
    print("Main launcher changed to com.x360nexus.MainActivity")
    print("Original XenDroid emulation host preserved")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--upstream", type=Path, required=True)
    p.add_argument("--nexus", type=Path, required=True)
    p.add_argument("--decoded", type=Path, required=True)
    p.add_argument("--license", type=Path, required=True)
    args = p.parse_args()
    integrate(args.upstream, args.nexus, args.decoded, args.license)


if __name__ == "__main__":
    main()
