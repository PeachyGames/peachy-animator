# Original native-v5 IK migration baseline

These eight Peach Bot files were retained byte-for-byte at commit
`775c7e31a99ec99e6dfed3f77661e5cccb7546b1`, before native6/IK changes.
Each JSON is actual `peachy-cli export` output built from that source, repeated
twice with identical bytes. The baseline CLI SHA-256 was
`8c73ff3b9f51a0a481e80134a4c6f52ff493d559b7105f933c8273957e606b17`.

The migration test compares full pretty-serialized export bytes for both these
v5 inputs and the current examples, requiring empty constraints to remain omitted
from Spine JSON. Original artwork is independently generated Peach Bot artwork;
these files contain no private artist input or official implementation.

Captured November 6, 2026 UTC. The local retained 20-file source manifest and
repeat-export report are in `.local-backups/ik-formats-impl-20261106T092459Z-00c89f/`.
