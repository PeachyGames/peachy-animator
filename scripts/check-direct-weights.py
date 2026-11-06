#!/usr/bin/env python3
"""Check explicit Add and Direct mesh weights using original Peach Bot data.

Build peachy-cli, then pass a fresh evidence directory:
  python3 scripts/check-direct-weights.py --evidence-dir /tmp/peachy-direct-check \
      --cli target/debug/peachy-cli --cargo /path/to/cargo

The standard-library runner reuses check-weight-authoring.py's process, immutable
source, pose and native/Spine/packed roundtrip checks. --cargo additionally runs
the Session Bind and Direct tests for pending poses, transactions and history.
No private artwork or official editor/runtime implementation is accessed.
"""
import argparse
import copy
from datetime import datetime, timezone
import importlib.util
import json
from pathlib import Path
import re
import struct
import sys


HELPER_PATH = Path(__file__).with_name("check-weight-authoring.py")
SPEC = importlib.util.spec_from_file_location("peachy_weight_acceptance", HELPER_PATH)
H = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = H
sys.dont_write_bytecode = True
SPEC.loader.exec_module(H)
ROOT, FIXTURES = H.ROOT, H.FIXTURES


def add(bone, locals_):
    return {"command": "addMeshBoneBinding", "attachment": H.KEY, "bone": bone,
            "pose": {"locals": locals_}}


def direct(vertex, bone, weight, locals_):
    return {"command": "setMeshBoneWeight", "attachment": H.KEY, "vertex": vertex,
            "bone": bone, "weight": weight, "pose": {"locals": locals_}}


def groups(document):
    return H.attachment(document)["vertices"]["influences"]


def bits(entries):
    """Compare native f32 data including signed zero, rather than JSON spelling."""
    return [(entry["bone"], struct.pack("!fff", *entry["position"], entry["weight"]))
            for entry in entries]


def saved_variant(check, document, name):
    path = check.evidence / "projects" / f"{name}.peachy"
    H.write_json(path, document)
    check.validate(path)
    return path


def check_added(check, before, after, bone):
    check.metadata(before, after)
    H.require(len(groups(before)) == len(groups(after)), "Add changed vertex count")
    for old, new in zip(groups(before), groups(after)):
        H.require(len(new) == len(old) + 1 and bits(new[:-1]) == bits(old),
                  "Add rewrote old binding order, weights or positions")
        H.require(new[-1]["bone"] == bone and bits(new[-1:])[0][1][-4:] == b"\0\0\0\0",
                  "Add did not append a positive-zero influence for the new bone")


def check_direct(check, before, after, vertex, bone, weight):
    check.metadata(before, after)
    old, new = groups(before)[vertex], groups(after)[vertex]
    chosen = next(i for i, entry in enumerate(old) if entry["bone"] == bone)
    others = sum(H.f32(entry["weight"]) for i, entry in enumerate(old)
                 if i != chosen and entry["weight"] > 0)
    expected = copy.deepcopy(old)
    target = H.f32(weight)
    for i, entry in enumerate(expected):
        if i == chosen:
            entry["weight"] = target
        elif entry["weight"] > 0:
            entry["weight"] = H.f32(H.f32(entry["weight"]) * (1.0 - target) / others)
    H.require(bits(new) == bits(expected), "Direct did not proportionally resize only positive recipients")
    for i, (a, b) in enumerate(zip(groups(before), groups(after))):
        if i != vertex:
            H.require(bits(a) == bits(b), f"Direct changed unselected vertex {i}")
    H.require(len(groups(before)) == len(groups(after)), "Direct changed vertex count")


def target_draw(scene):
    return next(draw for draw in scene["attachments"] if draw["attachment"] == H.KEY)


def compare_all_poses(check, before, after, label):
    document = H.read_json(before)
    for clip in document["animations"]:
        end = H.clip_end(clip)
        for time in sorted({0.0, end * 0.37, end, end + 0.25}):
            for skin in document["skins"]:
                check.compare(check.pose(before, clip["name"], time, skin),
                              check.pose(after, clip["name"], time, skin), f"{label} {clip['name']} {skin} {time}")


def imported_original(check, fixture):
    name = Path(fixture).stem
    source = check.prepare(fixture, f"prepared-{name}")
    exported = check.evidence / "exports" / f"original-{name}.json"
    check.run(["export", source, exported])
    H.require(json.loads(check.run(["inspect", exported]))["supported"], "Original export was unsupported")
    imported = check.evidence / "projects" / f"imported-{name}.peachy"
    check.run(["import", exported, imported])
    check.validate(imported)
    H.require(H.attachment(H.read_json(imported)) == H.attachment(H.read_json(source)),
              "Import changed original weighted payload")
    return imported


def exercise(check, sources):
    projects = check.evidence / "projects"
    projects.mkdir()
    (check.evidence / "exports").mkdir()
    for relative, data in sources.items():
        if relative.parts[0] == "images":
            H.write_bytes(projects / relative, data)
        if relative.suffix == ".peachy":
            check.validate(FIXTURES / relative)
    report = {"cases": [], "failures": []}
    source = check.prepare("mesh-contract.peachy", "scratch")
    original = H.read_json(source)
    a = next(i for i, bone in enumerate(original["bones"]) if bone["name"] == "body")
    b = next(i for i, bone in enumerate(original["bones"]) if bone["name"] == "head")
    pose = H.setup_locals(original)
    bound = check.apply(source, "scratch-bound-a", [H.bind_command(a, pose)])
    bound_doc = H.read_json(bound)
    H.require(all(len(g) == 1 and g[0]["bone"] == a and g[0]["weight"] == 1 for g in groups(bound_doc)),
              "Bind A1 did not create one full influence per vertex")
    check.metadata(original, bound_doc)
    check.compare(check.pose(source, H.SETUP_CLIP, 0), check.pose(bound, H.SETUP_CLIP, 0), "Scratch Bind")
    added = check.apply(bound, "scratch-added-b-zero", [add(b, pose)])
    added_doc = H.read_json(added)
    check_added(check, bound_doc, added_doc, b)
    compare_all_poses(check, bound, added, "Scratch Add0 shape")
    selected = 1
    edited = check.apply(added, "scratch-direct-b-thirty", [direct(selected, b, 0.3, pose)])
    edited_doc = H.read_json(edited)
    check_direct(check, added_doc, edited_doc, selected, b, 0.3)
    H.require([H.f32(e["weight"]) for e in groups(edited_doc)[selected]] == [H.f32(0.7), H.f32(0.3)],
              "Exact target vertex is not A.7/B.3")
    check.compare(check.pose(added, H.SETUP_CLIP, 0), check.pose(edited, H.SETUP_CLIP, 0), "Scratch Direct at bind pose")
    report["cases"].append({"case": "Unweighted → Bind A1 → Add B0 → Direct selected B.3/A.7",
                            "vertex": selected, "boneA": a, "boneB": b, **check.roundtrip(edited, "scratch-direct")})

    moved = copy.deepcopy(pose)
    moved[b]["x"] = H.f32(moved[b]["x"] + 10)
    moved_added = check.snapshot(added, "scratch-moved-before", moved)
    moved_direct = check.snapshot(edited, "scratch-moved-after", moved)
    before_scene = check.pose(moved_added, H.SETUP_CLIP, 0)
    after_scene = check.pose(moved_direct, H.SETUP_CLIP, 0)
    old_point = target_draw(before_scene)["vertices"][selected]
    new_point = target_draw(after_scene)["vertices"][selected]
    deformation = max(abs(x - y) for x, y in zip(old_point, new_point))
    H.require(deformation > check.tolerance, "Direct failed to deform the selected vertex in the moved pose")
    masked = copy.deepcopy(after_scene)
    target_draw(masked)["vertices"][selected] = copy.deepcopy(old_point)
    check.compare(before_scene, masked, "Only selected vertex deforms after moving B")
    report["cases"].append({"case": "Direct deformation is intentional; only selected vertex responds",
                            "maxSelectedCoordinateChange": deformation, **check.roundtrip(moved_direct, "scratch-moved-direct")})

    # Keep both ordinary limb orders. The codec must never silently sort the
    # child-before-parent order just to make a particular editor import succeed.
    hand = next(i for i, bone in enumerate(original["bones"]) if bone["name"] == "hand.R")
    forearm = next(i for i, bone in enumerate(original["bones"]) if bone["name"] == "forearm.R")
    for first, second in [(hand, forearm), (forearm, hand)]:
        name = f"scratch-order-{first}-{second}"
        order_bound = check.apply(source, name + "-bound", [H.bind_command(first, pose)])
        order_added = check.apply(order_bound, name + "-added", [add(second, pose)])
        check_added(check, H.read_json(order_bound), H.read_json(order_added), second)
        compare_all_poses(check, order_bound, order_added, name + " Add0 shape")
        order_edit = check.apply(order_added, name + "-direct", [direct(selected, second, 0.3, pose)])
        order_doc = H.read_json(order_edit)
        check_direct(check, H.read_json(order_added), order_doc, selected, second, 0.3)
        H.require(all([entry["bone"] for entry in group] == [first, second] for group in groups(order_doc)),
                  "Scratch limb binding order changed")
        report["cases"].append({"case": "Ordinary scratch limb binding order", "influenceOrder": [first, second],
                                "vertex": selected, **check.roundtrip(order_edit, name)})

    imported = {}
    for fixture in ["weighted-bot.peachy", "weighted-contract.peachy", "weighted-repeated.peachy"]:
        source_ = imported_original(check, fixture)
        before = H.read_json(source_)
        name = Path(fixture).stem
        imported[name] = source_
        new_bone = next(i for i in range(len(before["bones"]))
                        if all(entry["bone"] != i for g in groups(before) for entry in g))
        # Add must retain old nonunit sums, zeros and repeated entries verbatim.
        added_ = check.apply(source_, f"{name}-added-zero", [add(new_bone, H.setup_locals(before))])
        check_added(check, before, H.read_json(added_), new_bone)
        compare_all_poses(check, source_, added_, f"{name} Add0 shape")
        if name == "weighted-repeated":
            report["cases"].append({"case": "Add0 preserves repeated/zero/order data", **check.roundtrip(added_, name + "-add")})
            continue
        vertex = 2 if name == "weighted-contract" else 4
        entry = groups(before)[vertex][0]
        weight = entry["weight"] if name == "weighted-contract" else 0.6
        if name == "weighted-contract":
            H.require(abs(sum(e["weight"] for e in groups(before)[vertex]) - 1) > 1e-6,
                      "Expected an original nonunit selected vertex")
        result = check.apply(source_, f"{name}-direct", [direct(vertex, entry["bone"], weight, H.setup_locals(before))])
        after = H.read_json(result)
        check_direct(check, before, after, vertex, entry["bone"], weight)
        H.require(abs(sum(e["weight"] for e in groups(after)[vertex]) - 1) < 1e-6,
                  "Explicit Direct edit did not normalize the selected vertex")
        report["cases"].append({"case": name + " imported Direct (normalizes only explicit vertex)",
                                "vertex": vertex, **check.roundtrip(result, name + "-direct")})

    # Controlled inputs are derivatives of our original fixture, never artist data.
    signed = copy.deepcopy(H.read_json(imported["weighted-repeated"]))
    zero = next(e for e in groups(signed)[4] if e["weight"] == 0)
    zero["weight"] = -0.0
    zero["position"][0] = -0.0
    signed_path = saved_variant(check, signed, "signed-zero-and-unselected-repeat")
    signed_edit = check.apply(signed_path, "signed-zero-direct", [direct(0, 7, 0.6, H.setup_locals(signed))])
    check_direct(check, signed, H.read_json(signed_edit), 0, 7, 0.6)
    signed_add = check.apply(signed_path, "signed-zero-add", [add(4, H.setup_locals(signed))])
    check_added(check, signed, H.read_json(signed_add), 4)
    report["cases"].append({"case": "Signed-zero and unselected repeated payload stays bitwise intact",
                            "native": str(signed_edit.relative_to(check.evidence))})

    known = copy.deepcopy(added_doc)
    positions = copy.deepcopy(groups(known)[selected][0]["position"])
    groups(known)[selected] = [{"bone": bone, "position": positions, "weight": weight}
                              for bone, weight in [(a, 0.2), (b, 0.3), (0, 0.5)]]
    known_path = saved_variant(check, known, "known-vector")
    known_edit = check.apply(known_path, "known-vector-direct", [direct(selected, a, 0.6, pose)])
    check_direct(check, known, H.read_json(known_edit), selected, a, 0.6)
    vector = [H.f32(e["weight"]) for e in groups(H.read_json(known_edit))[selected]]
    H.require(all(abs(x - y) < 1e-7 for x, y in zip(vector, [0.6, 0.15, 0.25])),
              "Known .2/.3/.5 → .6/.15/.25 vector failed")
    report["cases"].append({"case": "Known proportional redistribution .2/.3/.5 → .6/.15/.25",
                            **check.roundtrip(known_edit, "known-vector-direct")})

    singular_direct = copy.deepcopy(pose)
    singular_direct[a]["scale_x"] = 0
    singular_direct[a]["scale_y"] = 0
    singular_success = check.apply(added, "direct-singular-pose-valid", [direct(selected, b, 0.3, singular_direct)])
    check_direct(check, added_doc, H.read_json(singular_success), selected, b, 0.3)
    sole_positive = check.apply(added, "direct-sole-positive-one-noop", [direct(selected, a, 1, pose)])
    H.require(H.read_json(sole_positive) == added_doc, "Sole-positive Direct 1 should be a validated no-op")
    report["cases"].append({"case": "Direct requires no inverse; sole-positive 1 is a no-op",
                            "native": str(singular_success.relative_to(check.evidence))})

    sparse = copy.deepcopy(bound_doc)
    groups(sparse)[-1].append({"bone": b, "position": [0, 0], "weight": 0})
    sparse_path = saved_variant(check, sparse, "sparse-zero-binding")
    singular_add = copy.deepcopy(pose)
    singular_add[b]["scale_x"] = 0
    precision_add = copy.deepcopy(pose)
    precision_add[b]["x"] = 1e8
    nonfinite = copy.deepcopy(added_doc)
    groups(nonfinite)[selected][0]["position"] = [H.f32(3.4e38), H.f32(3.4e38)]
    nonfinite_path = saved_variant(check, nonfinite, "finite-payload-overflow-candidate")
    overflow_pose = copy.deepcopy(pose)
    overflow_pose[a]["scale_x"] = 10
    bad_bone = {"name": "invalid batch hierarchy", "parent": len(pose) + 10, "length": 1,
                "setup": {"x": 0, "y": 0, "rotation": 0, "scale_x": 1, "scale_y": 1}}
    expanded_pose = copy.deepcopy(pose) + [bad_bone["setup"]]
    repeat_path = imported["weighted-repeated"]
    repeat_doc = H.read_json(repeat_path)
    failures = [
        (source, "add-unweighted", [add(b, pose)], "requires a weighted mesh"),
        (source, "direct-unweighted", [direct(selected, b, 0.3, pose)], "require a weighted mesh"),
        (repeat_path, "direct-repeated-selected", [direct(4, 8, 0.3, H.setup_locals(repeat_doc))], "ambiguous for repeated bones"),
        (sparse_path, "direct-sparse-missing", [direct(selected, b, 0.3, pose)], "no binding on this vertex"),
        (sparse_path, "add-existing-global-zero", [add(b, pose)], "already bound to this mesh"),
        (added, "add-existing-global", [add(b, pose)], "already bound to this mesh"),
        (added, "direct-zero-recipients", [direct(selected, a, 0.3, pose)], "without another positive recipient"),
        (bound, "add-singular-new-bone", [add(b, singular_add)], "singular bone transform"),
        (bound, "add-coordinate-precision", [add(b, precision_add)], "coordinate precision"),
        (nonfinite_path, "direct-nonfinite-candidate", [direct(selected, b, 0.3, overflow_pose)], "nonfinite world vertices"),
        (added, "direct-negative", [direct(selected, b, -0.1, pose)], "finite and in"),
        (added, "direct-over-one", [direct(selected, b, 1.1, pose)], "finite and in"),
        (added, "direct-invalid-pose", [direct(selected, b, 0.3, [])], "one finite local transform per bone"),
        (added, "direct-invalid-vertex", [direct(len(groups(added_doc)), b, 0.3, pose)], "Unknown mesh vertex"),
        (added, "direct-invalid-bone", [direct(selected, len(pose), 0.3, pose)], "Unknown weight bone"),
        (added, "direct-batch-atomic", [direct(selected, b, 0.3, pose), direct(selected, a, -0.1, pose)], "finite and in"),
        (bound, "add-batch-atomic", [add(b, pose), add(b, pose)], "already bound to this mesh"),
        (bound, "invalid-hierarchy-batch", [{"command": "addBone", "bone": bad_bone}, add(b, expanded_pose)], "valid bone hierarchy"),
    ]
    for input_, name, commands, reason in failures:
        check.apply(input_, name, commands, failure=True, expected_error=reason)
        report["failures"].append({"case": name, "expectedError": reason})
    check.apply(added, "existing-output", [direct(selected, b, 0.3, pose)], failure=True, existing=True,
                expected_error="Cannot publish new file")
    report["failures"].append({"case": "existing-output", "expectedError": "Cannot publish new file"})
    H.require(not list(projects.glob("*.tmp-*")), "CLI left temporary project files")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--evidence-dir", required=True, type=Path, help="New directory; an existing path is refused")
    parser.add_argument("--cli", type=Path, default=ROOT / "target" / "debug" / ("peachy-cli.exe" if sys.platform == "win32" else "peachy-cli"))
    parser.add_argument("--cargo", help="Optional Cargo executable/wrapper for actual Session Bind and Direct tests")
    args = parser.parse_args()
    evidence, cli = args.evidence_dir.expanduser().resolve(), args.cli.expanduser().resolve()
    H.require(cli.is_file(), f"Build peachy-cli first or pass --cli: {cli}")
    H.require(evidence != FIXTURES and FIXTURES not in evidence.parents,
              "Evidence cannot be written inside original fixtures")
    required = ["mesh-contract.peachy", "weighted-bot.peachy", "weighted-contract.peachy", "weighted-repeated.peachy"]
    H.require(all((FIXTURES / name).is_file() for name in required), "Original weighted fixtures are missing")
    evidence.mkdir(parents=True, exist_ok=False)
    sources = {}
    for source in sorted(FIXTURES.rglob("*")):
        if source.is_file():
            relative = source.relative_to(FIXTURES)
            H.require(not source.is_symlink(), f"Original fixture must not be a symlink: {source}")
            sources[relative] = source.read_bytes()
            H.write_bytes(evidence / "baseline" / relative, sources[relative])
    H.write_json(evidence / "source-manifest.json", {
        str(relative): {"sha256": H.digest(data), "bytes": len(data)} for relative, data in sources.items()})
    check = H.Acceptance(cli, evidence, 0.001)
    report = {"status": "failed", "started": datetime.now(timezone.utc).isoformat(),
              "cli": str(cli), "cliSHA256": H.file_digest(cli), "scriptSHA256": H.file_digest(Path(__file__)),
              "helperSHA256": H.file_digest(HELPER_PATH), "nativeFormat": 4, "tolerance": check.tolerance,
              "coreSessionTests": {"status": "not-run"}, "limitations": [
                  "Direct may deform geometry; scene checks compare edited data with its own adapter/packed roundtrip.",
                  "CLI creates new outputs; overwrite Save/backup and native pointer/keyboard behavior are not checked.",
                  "Explicit pose snapshots are not transient Session state; --cargo checks pending pose/history and fixed-baseline previews.",
                  "Our independent adapter is used; no external runtime or installed-editor acceptance is claimed.",
                  "Zero-recipient and sparse/repeated selection failures are bounded policies, not assertions about every Spine edge case.",
                  "No Update bindings, locks, multi-vertex Direct, brushes, Auto, Smooth or Prune acceptance is claimed."]}
    try:
        if args.cargo:
            results = {}
            for filter_, expected in [("binding_tests::", 13), ("direct_weight_tests::", 11)]:
                output = check.run(["test", "-p", "peachy-engine", "--offline", filter_, "--", "--nocapture"], executable=args.cargo)
                passed = sum(map(int, re.findall(r"test result: ok\. (\d+) passed;", output)))
                H.require(passed >= expected, f"Cargo did not run the expected {filter_} semantic tests")
                results[filter_] = passed
            report["coreSessionTests"] = {"status": "passed", "tests": sum(results.values()), "suites": results}
        report.update(exercise(check, sources))
        report["status"] = "passed"
    except Exception as error:
        report["error"] = str(error)
    finally:
        unchanged = H.unchanged_sources(sources, evidence / "baseline")
        report["sourceAndBaselineBytesUnchanged"] = unchanged
        if not unchanged:
            report.update(status="failed", error="Original source or retained baseline bytes changed")
        report.update(metrics=check.metrics, processInvocations=len(check.invocations), sourceFiles=len(sources),
                      finished=datetime.now(timezone.utc).isoformat())
        H.write_json(evidence / "invocations.json", check.invocations)
        H.write_json(evidence / "report.json", report)
        lines = ["# Direct weights CLI acceptance", "", f"Status: {report['status']}", "",
                 f"Original source and retained baseline bytes unchanged: {unchanged}.",
                 f"Scene comparisons: {check.metrics['sceneComparisons']}; maximum vertex error: {check.metrics['maxVertexError']}.",
                 "Cases, commands, process output and source hashes are retained with this report.", "", "## Limits", "",
                 *[f"- {limit}" for limit in report["limitations"]]]
        if report.get("error"):
            lines += ["", f"Failure: {report['error']}"]
        H.write_bytes(evidence / "REPORT.md", ("\n".join(lines) + "\n").encode())
    print(json.dumps({"status": report["status"], "report": str(evidence / "report.json"),
                      "sourcesUnchanged": unchanged, "metrics": check.metrics}, indent=2))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, RuntimeError) as error:
        sys.exit(str(error))
