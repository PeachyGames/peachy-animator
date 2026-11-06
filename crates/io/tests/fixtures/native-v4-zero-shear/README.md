# Original native v4 and zero-shear export goldens

These seven `.peachy` files are the unchanged original Team Peachy examples at
`e282f14f9aedc46c4ffda3c0c404fddc5fb11894`, before native v5 or adapter changes.
Their paired `.json` files are actual outputs from a freshly built CLI at that
same commit, captured November 6, 2026 UTC. They contain our original rig data;
no artist files, artwork, editor projects or proprietary implementation are copied.

Keep the native files at v4 and the exports unchanged. The migration test checks
the complete exported JSON payload against these independent pre-change outputs,
including skins, curves, weighted influence order, zeros and nonunit totals. It
also checks the seven current zero-shear examples against the same export goldens.
Do not regenerate these historical inputs with the latest model or exporter.
