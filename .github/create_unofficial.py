import shutil, os, subprocess, zipfile, json, sys

from itertools import chain

PROJECT_PATH = sys.argv[1]
INTERNAL_NAME = sys.argv[2]
OFFICIAL_ZIP = f"{PROJECT_PATH}/bin/x64/Release/{INTERNAL_NAME}/latest.zip"
UNOFFICIAL_ZIP = f"{PROJECT_PATH}/bin/x64/Release/{INTERNAL_NAME}/latestUnofficial.zip"

shutil.copy(OFFICIAL_ZIP, UNOFFICIAL_ZIP)

subprocess.check_call(['7z', 'd', UNOFFICIAL_ZIP, f"{INTERNAL_NAME}.json"])

with zipfile.ZipFile(UNOFFICIAL_ZIP) as file:
    members = [member for member in file.namelist() if member in (f"{INTERNAL_NAME}.dll", f"{INTERNAL_NAME}.deps.json", f"{INTERNAL_NAME}.json", f"{INTERNAL_NAME}.pdb")]

subprocess.check_call(['7z', 'rn', UNOFFICIAL_ZIP] + list(chain.from_iterable((m, m.replace(INTERNAL_NAME, f"{INTERNAL_NAME}Unofficial")) for m in members)))

with open(f"{PROJECT_PATH}/bin/x64/Release/{INTERNAL_NAME}/{INTERNAL_NAME}.json") as file:
    manifest = json.load(file)

manifest['Punchline'] = f"Unofficial/uncertified build of {manifest['Name']}. {manifest['Punchline']}"
manifest['InternalName'] += 'Unofficial'
manifest['Name'] += ' (Unofficial)'
manifest['IconUrl'] = f"https://raw.githubusercontent.com/WorkingRobot/MyDalamudPlugins/main/icons/{manifest['InternalName']}.png"

with zipfile.ZipFile(UNOFFICIAL_ZIP, "a", zipfile.ZIP_DEFLATED, compresslevel = 7) as file:
    file.writestr(f"{INTERNAL_NAME}Unofficial.json", json.dumps(manifest, indent = 2))