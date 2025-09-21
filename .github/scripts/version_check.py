import sys
import tomllib
import urllib.request
import json
from packaging.version import Version

pkg = tomllib.load(open("Cargo.toml", "rb"))["package"]
loc, name = Version(pkg["version"]), pkg["name"]
crate_url = f"https://crates.io/api/v1/crates/{name}"
json_data = json.load(urllib.request.urlopen(crate_url))
rem = Version(json_data["crate"]["max_stable_version"])
print(f"Crate: {name}")
print(f"Local: {loc}")
print(f"Remote: {rem}")

if loc <= rem:
    sys.exit(f"❌ {loc} <= {rem}, bump Cargo.toml")

print(f"✅ {loc} > {rem}")
