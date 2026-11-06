#!/usr/bin/env python3
"""Check serialized initial Bind/Remove-all with original Peach Bot fixtures.

Build peachy-cli first, then supply a fresh evidence directory:
  python3 scripts/check-weight-authoring.py --evidence-dir /tmp/peachy-bind-check \
      --cli target/debug/peachy-cli --cargo /path/to/cargo

Only the Python standard library and the supplied CLI are required. --cargo also
runs the Session regression tests, which cover actual pending poses and history.
The CLI has no transient Session interface or overwrite-save operation: its
explicit posed snapshots and native create/load checks do not claim those features.
No official editor/runtime implementation or private artwork is accessed.
"""
import argparse
import copy
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import re
import struct
import subprocess
import sys
from native_schema import NATIVE_FORMAT, bone_transform


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "examples" / "peach-bot"
SCHEMA_HELPER = Path(__file__).with_name("native_schema.py")
SETUP_CLIP = "weight authoring setup"
KEY = {"skin": "default", "slot": "hand.R", "placeholder": "hand.R"}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def write_bytes(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as output:
        output.write(data)


def write_json(path, value):
    write_bytes(path, (json.dumps(value, indent=2, allow_nan=False) + "\n").encode())


def read_json(path):
    return json.loads(path.read_text())


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    result = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


def f32(value):
    return struct.unpack("!f", struct.pack("!f", value))[0]


def attachment(document):
    return document["skins"][KEY["skin"]][KEY["slot"]][KEY["placeholder"]]


def setup_locals(document):
    return [copy.deepcopy(bone["setup"]) for bone in document["bones"]]


def final_key_locals(document, clip):
    """Capture a held end pose; this deliberately does not implement interpolation."""
    locals_ = setup_locals(document)
    fields = {"x": "x", "y": "y", "rotation": "rotation", "scaleX": "scale_x", "scaleY": "scale_y",
              "shearX": "shear_x", "shearY": "shear_y"}
    for track in clip["tracks"]:
        if not track["keys"]:
            continue
        field = fields[track["property"]]
        base = f32(document["bones"][track["bone"]]["setup"][field])
        value = f32(track["keys"][-1]["value"])
        locals_[track["bone"]][field] = f32(base * value if field.startswith("scale_") else base + value)
    return locals_


def clip_end(clip):
    return max((key["time"] for track in clip["tracks"] for key in track["keys"]), default=0.0)


def bind_command(bone, locals_):
    return {"command": "bindMeshToBone", "attachment": KEY, "bone": bone, "pose": {"locals": locals_}}


def unbind_command(locals_):
    return {"command": "unbindMesh", "attachment": KEY, "pose": {"locals": locals_}}


class Acceptance:
    def __init__(self, cli, evidence, tolerance):
        self.cli = cli
        self.evidence = evidence
        self.tolerance = tolerance
        self.invocations = []
        self.scene_cache = {}
        self.metrics = {"sceneComparisons": 0, "worldCoordinates": 0, "vertexCoordinates": 0,
                        "maxWorldError": 0.0, "maxVertexError": 0.0, "maxUVError": 0.0}

    def run(self, arguments, failure=False, executable=None, expected_error=None):
        argv = [str(executable or self.cli), *map(str, arguments)]
        result = subprocess.run(argv, cwd=ROOT, text=True, capture_output=True, timeout=60, check=False)
        number = len(self.invocations) + 1
        stdout = Path("logs") / f"{number:04}.stdout"
        stderr = Path("logs") / f"{number:04}.stderr"
        write_bytes(self.evidence / stdout, result.stdout.encode())
        write_bytes(self.evidence / stderr, result.stderr.encode())
        self.invocations.append({"argv": argv, "returncode": result.returncode,
                                 "expectedFailure": failure, "stdout": str(stdout), "stderr": str(stderr)})
        require((result.returncode != 0) if failure else (result.returncode == 0),
                f"Unexpected CLI result for {argv}: {result.stderr.strip()}")
        if expected_error:
            require(expected_error in result.stderr,
                    f"Expected {expected_error!r} for {argv}, got {result.stderr.strip()!r}")
        return result.stdout

    def validate(self, source):
        require(json.loads(self.run(["validate", source]))["valid"], f"Invalid native project: {source}")
        # The supplied CLI must actually decode the current strict schema, not
        # merely accept an older input. Successful writes are checked here too.
        require(read_json(source)["format"] == NATIVE_FORMAT, f"Expected native v{NATIVE_FORMAT}: {source}")

    def apply(self, source, name, commands, failure=False, existing=False, expected_error=None):
        command_path = self.evidence / "commands" / f"{name}.json"
        output = self.evidence / "projects" / f"{name}.peachy"
        write_json(command_path, commands)
        before = source.read_bytes()
        if existing:
            write_bytes(output, before)
        self.run(["apply", source, command_path, output], failure=failure, expected_error=expected_error)
        require(source.read_bytes() == before, f"CLI changed its input bytes: {source}")
        if failure:
            require(output.read_bytes() == before if existing else not output.exists(),
                    f"Failed apply changed/published output: {output}")
            return None
        self.validate(output)
        return output

    def prepare(self, fixture, name):
        source = self.evidence / "baseline" / fixture
        require(all(clip["name"] != SETUP_CLIP for clip in read_json(source)["animations"]),
                "Acceptance setup clip already exists")
        return self.apply(source, name, [{"command": "addAnimation", "name": SETUP_CLIP, "duration": 0}])

    def pose(self, source, animation, time, skin="default"):
        key = (str(source), animation, time, skin)
        if key not in self.scene_cache:
            self.scene_cache[key] = json.loads(self.run(["pose", source, animation, time, skin]))
        return self.scene_cache[key]

    def compare(self, before, after, label):
        require([bone["name"] for bone in before["bones"]] == [bone["name"] for bone in after["bones"]],
                f"{label}: bone identity changed")
        for a, b in zip(before["bones"], after["bones"]):
            for field in ["a", "b", "c", "d", "x", "y"]:
                error = abs(a["world"][field] - b["world"][field])
                require(math.isfinite(error) and error <= self.tolerance, f"{label}: bone {a['name']} {field}: {error}")
                self.metrics["worldCoordinates"] += 1
                self.metrics["maxWorldError"] = max(error, self.metrics["maxWorldError"])
        identity = lambda draw: (draw["slot"], draw["attachment"]["skin"], draw["attachment"]["slot"], draw["attachment"]["placeholder"])
        a_draws = {identity(draw): draw for draw in before["attachments"]}
        b_draws = {identity(draw): draw for draw in after["attachments"]}
        require(a_draws.keys() == b_draws.keys(), f"{label}: attachment identities changed")
        for key, a in a_draws.items():
            b = b_draws[key]
            for field in ["image", "triangles", "hull"]:
                require(a[field] == b[field], f"{label}: {field} changed for {key}")
            for field, tolerance, metric in [("vertices", self.tolerance, "maxVertexError"), ("uvs", 1e-6, "maxUVError")]:
                require(len(a[field]) == len(b[field]), f"{label}: {field} count changed for {key}")
                for a_point, b_point in zip(a[field], b[field]):
                    for a_value, b_value in zip(a_point, b_point):
                        error = abs(a_value - b_value)
                        require(math.isfinite(error) and error <= tolerance, f"{label}: {field} error {error} for {key}")
                        self.metrics[metric] = max(error, self.metrics[metric])
                        if field == "vertices":
                            self.metrics["vertexCoordinates"] += 1
        self.metrics["sceneComparisons"] += 1

    def metadata(self, before, after):
        expected = copy.deepcopy(before)
        attachment(expected)["vertices"] = copy.deepcopy(attachment(after)["vertices"])
        require(after == expected, "Bind/Unbind changed data outside the addressed mesh vertices")

    def snapshot(self, source, name, locals_):
        document = read_json(source)
        require(len(locals_) == len(document["bones"]), "Incomplete explicit pose snapshot")
        for bone, local in zip(document["bones"], locals_):
            bone["setup"] = copy.deepcopy(local)
        path = self.evidence / "projects" / f"{name}.peachy"
        write_json(path, document)
        self.validate(path)
        return path

    def roundtrip(self, source, name):
        document = read_json(source)
        exported = self.evidence / "exports" / f"{name}.json"
        self.run(["export", source, exported])
        inspection = json.loads(self.run(["inspect", exported]))
        require(inspection["supported"], f"Export inspection failed: {inspection}")
        reopened = self.evidence / "projects" / f"{name}-spine.peachy"
        self.run(["import", exported, reopened])
        self.validate(reopened)
        require(attachment(read_json(reopened)) == attachment(document), "Spine roundtrip changed mesh payload")
        parent = self.evidence / "bundles" / name
        parent.mkdir(parents=True)
        packed = Path(self.run(["bundle", source, parent]).strip()).resolve()
        require(packed.parent == parent and packed.is_dir(), "Bundle escaped its evidence parent")
        require(read_json(packed / "character.json") == read_json(exported), "Packed and standalone JSON differ")
        png = (packed / "character.png").read_bytes()
        require(png[:8] == b"\x89PNG\r\n\x1a\n", "Packed atlas image is not PNG")
        width, height = struct.unpack(">II", png[16:24])
        atlas = (packed / "character.atlas").read_text()
        require(f"size: {width},{height}\n" in atlas, "Atlas dimensions disagree with PNG")
        paths = {item["path"] for skin in document["skins"].values() for slot in skin.values() for item in slot.values()}
        require(all(f"{path}\n  bounds:" in atlas for path in paths), "Packed atlas omits a source image")
        packed_native = self.evidence / "projects" / f"{name}-packed.peachy"
        self.run(["import", packed / "character.json", packed_native])
        self.validate(packed_native)
        for clip in document["animations"]:
            end = clip_end(clip)
            for time in sorted({0.0, end * 0.37, end, end + 0.25}):
                for skin in document["skins"]:
                    before = self.pose(source, clip["name"], time, skin)
                    self.compare(before, self.pose(reopened, clip["name"], time, skin), f"{name} Spine {clip['name']} {skin} {time}")
                    self.compare(before, self.pose(packed_native, clip["name"], time, skin), f"{name} packed {clip['name']} {skin} {time}")
        return {"native": str(source.relative_to(self.evidence)), "export": str(exported.relative_to(self.evidence)),
                "bundle": str(packed.relative_to(self.evidence)), "imageCount": len(paths)}


def exercise(check, sources):
    projects = check.evidence / "projects"
    exports = check.evidence / "exports"
    projects.mkdir()
    exports.mkdir()
    for relative, data in sources.items():
        if relative.parts[0] == "images":
            write_bytes(projects / relative, data)
    for relative in sorted(sources):
        if relative.suffix == ".peachy":
            check.validate(FIXTURES / relative)

    source = check.prepare("mesh-contract.peachy", "prepared-reflected")
    document = read_json(source)
    target = next(i for i, bone in enumerate(document["bones"]) if bone["name"] == "body")
    slot = next(item for item in document["slots"] if item["name"] == KEY["slot"])
    require(target != slot["bone"], "Acceptance bind bone must differ from slot bone")
    require(document["bones"][0]["setup"]["scale_x"] < 0, "Expected reflected original contract")
    require(abs(document["bones"][0]["setup"]["scale_x"]) != document["bones"][0]["setup"]["scale_y"], "Expected nonuniform original contract")
    report = {"cases": [], "failures": []}

    locals_ = setup_locals(document)
    setup_bound = check.apply(source, "bound-setup", [bind_command(target, locals_)])
    check.metadata(document, read_json(setup_bound))
    entries = attachment(read_json(setup_bound))["vertices"]["influences"]
    require(all(len(vertex) == 1 and vertex[0]["bone"] == target and vertex[0]["weight"] == 1 for vertex in entries),
            "Initial Bind did not create exactly one full influence per vertex")
    check.compare(check.pose(source, SETUP_CLIP, 0), check.pose(setup_bound, SETUP_CLIP, 0), "Setup Bind")
    report["cases"].append({"case": "Setup Bind: reflected/nonuniform, different slot bone", **check.roundtrip(setup_bound, "bound-setup")})

    clip = document["animations"][0]
    time = clip_end(clip) + 0.137
    animate = final_key_locals(document, clip)
    animation_snapshot = check.snapshot(source, "held-animation-snapshot", animate)
    check.compare(check.pose(source, clip["name"], time), check.pose(animation_snapshot, SETUP_CLIP, 0), "Exact held animation snapshot")
    animated_bound = check.apply(source, "bound-animate", [bind_command(target, animate)])
    check.metadata(document, read_json(animated_bound))
    check.compare(check.pose(source, clip["name"], time), check.pose(animated_bound, clip["name"], time), "Animate Bind")
    report["cases"].append({"case": "Animate held-pose Bind", "time": time, **check.roundtrip(animated_bound, "bound-animate")})

    pending = copy.deepcopy(animate)
    pending[target]["x"] = f32(pending[target]["x"] + 23)
    pending[target]["rotation"] = f32(pending[target]["rotation"] + 17)
    pending_before = check.snapshot(source, "explicit-posed-before", pending)
    pending_bound = check.apply(source, "bound-explicit-pose", [bind_command(target, pending)])
    check.metadata(document, read_json(pending_bound))
    pending_after = check.snapshot(pending_bound, "explicit-posed-after", pending)
    check.compare(check.pose(pending_before, SETUP_CLIP, 0), check.pose(pending_after, SETUP_CLIP, 0), "Explicit posed Bind snapshot")
    report["cases"].append({"case": "Explicit posed snapshot Bind (Session pending behavior is checked by core tests)",
                            **check.roundtrip(pending_bound, "bound-explicit-pose")})

    for fixture in ["weighted-bot.peachy", "weighted-contract.peachy", "weighted-repeated.peachy"]:
        name = Path(fixture).stem
        weighted = check.prepare(fixture, f"prepared-{name}")
        before = read_json(weighted)
        influences = attachment(before)["vertices"]["influences"]
        require(any(len(vertex) > 1 for vertex in influences), "Expected an original multibone weighted fixture")
        removed = check.apply(weighted, f"removed-{name}", [unbind_command(setup_locals(before))])
        after = read_json(removed)
        require(attachment(after)["vertices"]["mode"] == "slotLocal", "Remove-all did not remove bindings")
        check.metadata(before, after)
        check.compare(check.pose(weighted, SETUP_CLIP, 0), check.pose(removed, SETUP_CLIP, 0), f"{name} Remove-all")
        report["cases"].append({"case": f"Remove-all {name}", **check.roundtrip(removed, f"removed-{name}")})

    noop = check.apply(source, "unweighted-noop", [unbind_command(locals_)])
    require(read_json(noop) == document, "Unweighted Remove-all changed native data")
    report["cases"].append({"case": "Validated unweighted Remove-all no-op", "native": str(noop.relative_to(check.evidence))})
    singular_bind = copy.deepcopy(locals_)
    singular_bind[target]["scale_x"] = 0
    singular_unbind = copy.deepcopy(locals_)
    singular_unbind[slot["bone"]]["scale_y"] = 0
    precision_bind = copy.deepcopy(locals_)
    separate_bone = next(i for i, bone in enumerate(document["bones"]) if bone["name"] == "head")
    precision_bind[separate_bone]["x"] = 1e8
    precision_unbind = copy.deepcopy(locals_)
    precision_unbind[slot["bone"]]["x"] = 1e8
    failures = [
        (source, "singular-bind", [bind_command(target, singular_bind)], "singular bone transform"),
        (setup_bound, "singular-unbind", [unbind_command(singular_unbind)], "singular bone transform"),
        (setup_bound, "already-weighted-bind", [bind_command(target, locals_)], "Initial binding requires an unweighted mesh"),
        (source, "invalid-pose", [bind_command(target, [])], "one finite local transform per bone"),
        (source, "invalid-noop-pose", [unbind_command([])], "one finite local transform per bone"),
        (source, "invalid-bone-batch", [{"command": "setBoneLength", "bone": 0, "length": 123}, bind_command(len(document["bones"]), locals_)], "Unknown bind bone"),
        (source, "coordinate-precision-bind", [bind_command(separate_bone, precision_bind)], "coordinate precision"),
        (setup_bound, "coordinate-precision-unbind", [unbind_command(precision_unbind)], "coordinate precision"),
    ]
    for input_, name, commands, expected_error in failures:
        check.apply(input_, name, commands, failure=True, expected_error=expected_error)
        report["failures"].append(name)
    check.apply(source, "existing-output", [bind_command(target, locals_)], failure=True, existing=True,
                expected_error="Cannot publish new file")
    report["failures"].append("existing-output-refused-with-bytes-preserved")
    require(not list(projects.glob("*.tmp-*")), "CLI left temporary project files")
    return report


def unchanged_sources(sources, baseline):
    try:
        expected = set(sources)
        actual = {path.relative_to(FIXTURES) for path in FIXTURES.rglob("*") if path.is_file()}
        retained = {path.relative_to(baseline) for path in baseline.rglob("*") if path.is_file()}
        return expected == actual == retained and all(
            (FIXTURES / relative).read_bytes() == data and (baseline / relative).read_bytes() == data
            for relative, data in sources.items()
        )
    except OSError:
        return False


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--evidence-dir", required=True, type=Path, help="New directory; an existing path is refused")
    parser.add_argument("--cli", type=Path, default=ROOT / "target" / "debug" / ("peachy-cli.exe" if sys.platform == "win32" else "peachy-cli"))
    parser.add_argument("--cargo", help="Optional Cargo executable/wrapper for actual Session binding tests")
    parser.add_argument("--tolerance", type=float, default=0.001)
    args = parser.parse_args()
    evidence = args.evidence_dir.expanduser().resolve()
    cli = args.cli.expanduser().resolve()
    require(cli.is_file(), f"Build peachy-cli first or pass --cli: {cli}")
    require(math.isfinite(args.tolerance) and args.tolerance > 0, "Tolerance must be finite and positive")
    require(evidence != FIXTURES and FIXTURES not in evidence.parents, "Evidence cannot be written inside original fixtures")
    required = ["mesh-contract.peachy", "weighted-bot.peachy", "weighted-contract.peachy", "weighted-repeated.peachy"]
    require(all((FIXTURES / name).is_file() for name in required), "Original weighted fixtures are missing; use an up-to-date checkout")
    evidence.mkdir(parents=True, exist_ok=False)
    sources = {}
    for source in sorted(FIXTURES.rglob("*")):
        if source.is_file():
            relative = source.relative_to(FIXTURES)
            require(not source.is_symlink(), f"Original fixture source must not be a symlink: {source}")
            sources[relative] = source.read_bytes()
            write_bytes(evidence / "baseline" / relative, sources[relative])
    manifest = {str(relative): {"sha256": digest(data), "bytes": len(data)} for relative, data in sources.items()}
    write_json(evidence / "source-manifest.json", manifest)
    check = Acceptance(cli, evidence, args.tolerance)
    report = {"status": "failed", "started": datetime.now(timezone.utc).isoformat(),
              "cli": str(cli), "cliSHA256": file_digest(cli), "scriptSHA256": file_digest(Path(__file__)),
              "schemaHelperSHA256": file_digest(SCHEMA_HELPER),
              "nativeFormat": NATIVE_FORMAT, "tolerance": args.tolerance, "coreSessionTests": {"status": "not-run"},
              "limitations": ["CLI creates new native outputs; overwrite Save/backup behavior is not exercised.",
                              "Explicit posed CLI snapshots are not transient Session state; --cargo checks pending pose/history regressions.",
                              "Spine JSON roundtrips use our independent adapter; no installed-editor or external-runtime acceptance is claimed.",
                              "Only initial one-bone Bind and Remove-all are checked; no Direct, normalization, Update or Auto weighting."]}
    try:
        if args.cargo:
            output = check.run(["test", "-p", "peachy-engine", "--offline", "binding_tests::", "--", "--nocapture"], executable=args.cargo)
            passed = sum(map(int, re.findall(r"test result: ok\. (\d+) passed;", output)))
            require(passed > 0, "Cargo did not run any Session binding tests")
            report["coreSessionTests"] = {"status": "passed", "tests": passed}
        report.update(exercise(check, sources))
        report["status"] = "passed"
    except Exception as error:
        report["error"] = str(error)
    finally:
        unchanged = unchanged_sources(sources, evidence / "baseline")
        report["sourceAndBaselineBytesUnchanged"] = unchanged
        if not unchanged:
            report["status"] = "failed"
            report["error"] = "Original source or retained baseline bytes changed"
        report["metrics"] = check.metrics
        report["processInvocations"] = len(check.invocations)
        report["sourceFiles"] = len(sources)
        report["finished"] = datetime.now(timezone.utc).isoformat()
        write_json(evidence / "invocations.json", check.invocations)
        write_json(evidence / "report.json", report)
        lines = ["# Weight authoring CLI acceptance", "", f"Status: {report['status']}", "",
                 f"Original source and retained baseline bytes unchanged: {unchanged}.",
                 f"Process invocations: {len(check.invocations)}. Scene comparisons: {check.metrics['sceneComparisons']}.",
                 f"Maximum vertex error: {check.metrics['maxVertexError']}; tolerance: {args.tolerance}.", "",
                 "Cases, exact command payloads, process output and source hashes are retained alongside this report.", "",
                 "## Limits", "", *[f"- {limit}" for limit in report["limitations"]]]
        if report.get("error"):
            lines += ["", f"Failure: {report['error']}"]
        write_bytes(evidence / "REPORT.md", ("\n".join(lines) + "\n").encode())
    print(json.dumps({"status": report["status"], "report": str(evidence / "report.json"),
                      "sourcesUnchanged": unchanged, "metrics": check.metrics}, indent=2))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, RuntimeError) as error:
        sys.exit(str(error))
