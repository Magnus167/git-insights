import sys
import tomllib
import urllib.request
import urllib.error
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
    print(f"❌ {loc} <= {rem}, bump Cargo.toml")
    sys.exit(1)

print(f"✅ {loc} > {rem}")
# make the same check on pypi
pypi_url = f"https://pypi.org/pypi/{name}/json"
try:
    json_data = json.load(urllib.request.urlopen(pypi_url))
    rem = Version(json_data["info"]["version"])
    print(f"PyPI: {name}")
    print(f"Local: {loc}")
    print(f"Remote: {rem}")

    if loc <= rem:
        print(f"❌ {loc} <= {rem}, bump Cargo.toml")
        sys.exit(1)

    print(f"✅ {loc} > {rem}")
except urllib.error.HTTPError as e:
    if e.code == 404:
        print(f"✅ {name} not found on PyPI, good to publish")
    else:
        raise e
