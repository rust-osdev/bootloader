import toml
import requests
import subprocess

cargo_toml = toml.load("Cargo.toml")
crate_version = cargo_toml["workspace"]["package"]["version"]
print("Detected crate version " + crate_version)

api_url = "https://crates.io/api/v1/crates/bootloader/" + crate_version
released_version = requests.get(api_url).json()

if "version" in released_version:
    version = released_version["version"]
    assert (version["crate"] == "bootloader")
    assert (version["num"] == crate_version)
    print("Version " + crate_version + " already exists on crates.io")

else:
    print("Could not find version " + crate_version +
          " on crates.io; creating a new release")

    tag_name = "v" + crate_version
    print("  Tagging commit as " + tag_name)
    sha = subprocess.run(["git", "rev-parse", "HEAD"], check=True,
                         stdout=subprocess.PIPE).stdout.decode("utf-8").strip()
    subprocess.run([
        "gh", "api", "/repos/rust-osdev/x86_64/git/refs",
        "-X", "POST", "-H", "Accept: application/vnd.github.v3+json",
        "-F", "ref=refs/tags/" + tag_name,
        "-F", "sha="+sha
    ])

    subprocess.run([
        "gh", "api", "--method", "POST", "-H", "Accept: application/vnd.github+json",
        "/repos/rust-osdev/bootloader/releases",
        "-f", f"tag_name='{tag_name}'", "-f", f"target_commitish='{sha}'",
        "-f", f"name='{tag_name}'",
        "-f", "body='[Changelog](https://github.com/rust-osdev/bootloader/blob/main/Changelog.md)'",
        "-F", "draft=false", "-F", "prerelease=false", "-F", "generate_release_notes=false",
    ])

    print("  Done")
