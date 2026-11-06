"""Small shared native-schema facts for our fixture and CLI scripts.

The public document constant remains the source of the current project version.
This module reads our own source; no generated framework or external dependency
is needed. Bone and region constructors deliberately have different shapes.
"""
from pathlib import Path
import re


def native_format_version():
    source = Path(__file__).resolve().parents[1] / "crates/document/src/lib.rs"
    matches = re.findall(r"(?m)^pub const FORMAT_VERSION:\s*u32\s*=\s*(\d+)\s*;\s*$", source.read_text())
    if len(matches) != 1 or not 0 < int(matches[0]) < 2**32:
        raise RuntimeError("Expected one public u32 document FORMAT_VERSION constant")
    return int(matches[0])


NATIVE_FORMAT = native_format_version()


def region_transform(x=0, y=0, rotation=0):
    return dict(x=x, y=y, rotation=rotation, scale_x=1, scale_y=1)


def bone_transform(x=0, y=0, rotation=0):
    return {**region_transform(x, y, rotation), "shear_x": 0.0, "shear_y": 0.0}


def require_fk_document(document, purpose):
    """Our authored-only snapshot helpers are not a constraint evaluator.

    CLI pose exposes separate locals/appliedLocals. A future constrained helper
    must capture appliedLocals explicitly and reconstruct that snapshot via FK
    once, without applying the live constraint list a second time.
    """
    if document.get("constraints"):
        raise RuntimeError(f"{purpose} requires an unconstrained document; use an explicit checked applied pose")


def animation(name, tracks=(), ik_tracks=()):
    """Construct the current required animation shape with independent lists."""
    return dict(name=name, tracks=list(tracks), ik_tracks=list(ik_tracks))
