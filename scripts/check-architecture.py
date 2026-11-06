#!/usr/bin/env python3
"""Check actual Cargo dependencies against the functional-core/shell boundary."""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
# Direct production/build dependencies. Test-only tools cannot enter production
# through this rule; new capabilities require an explicit rule change.
RULES = {
    "peachy-document": {"serde"},
    "peachy-engine": {"peachy-document", "serde"},
    "peachy-spine": {"peachy-document", "serde", "serde_json"},
    "peachy-io": {"peachy-document", "peachy-spine", "serde_json", "image"},
    "peachy-cli": {"peachy-document", "peachy-engine", "peachy-spine", "peachy-io", "serde_json"},
    "peachy-mcp": {"peachy-document", "peachy-engine", "peachy-spine", "peachy-io", "serde", "serde_json", "rmcp", "schemars", "tokio"},
    "peachy-animator": {"peachy-document", "peachy-engine", "peachy-io", "serde_json", "eframe", "image", "rfd", "objc2", "objc2-app-kit", "objc2-foundation"},
}


def violations(metadata):
    members = set(metadata["workspace_members"])
    errors = []
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        name = package["name"]
        if name not in RULES:
            errors.append(f"{name}: declare a dependency layer before adding a crate")
            continue
        for dependency in package["dependencies"]:
            if dependency["kind"] != "dev" and dependency["name"] not in RULES[name]:
                errors.append(f"{name} -> {dependency['name']}: dependency crosses an architectural boundary")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cargo", default="cargo")
    args = parser.parse_args()
    cargo = str(Path(args.cargo).absolute()) if "/" in args.cargo else args.cargo
    metadata = json.loads(subprocess.check_output(
        [cargo, "metadata", "--no-deps", "--format-version", "1", "--offline"], cwd=ROOT, text=True))
    errors = violations(metadata)
    # A deliberately invalid edge exercises the rule without changing a manifest.
    probe = {"workspace_members": ["probe"], "packages": [{"id": "probe", "name": "peachy-engine", "dependencies": [{"name": "eframe", "kind": None}]}]}
    if not violations(probe):
        raise SystemExit("Architecture rule failed to reject a UI dependency in the engine")
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"Architecture boundaries pass for {len(metadata['workspace_members'])} crates.")


if __name__ == "__main__":
    main()
