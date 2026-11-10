// Usage: node scripts/check-rig-reexport.mjs <runtime index.js> <atlas> <original.json> <reexported.json> [seconds]
// Run two skeleton JSON files through the external runtime with physics
// simulation on, frame by frame, and report the largest world/vertex difference.
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const [runtime, atlasPath, originalPath, candidatePath, seconds = '6'] = process.argv.slice(2);
const spine = await import(pathToFileURL(runtime));
class T extends spine.Texture { constructor(w, h) { super({ width: w, height: h }); } setFilters() {} setWraps() {} dispose() {} }
const atlas = new spine.TextureAtlas(readFileSync(atlasPath, 'utf8'));
for (const page of atlas.pages) page.setTexture(new T(page.width, page.height));
const load = path => new spine.SkeletonJson(new spine.AtlasAttachmentLoader(atlas)).readSkeletonData(readFileSync(path, 'utf8'));
const [a, b] = [load(originalPath), load(candidatePath)];
const report = { animations: [], frames: 0, boneValues: 0, vertexValues: 0, maxBoneDiff: 0, maxVertexDiff: 0, physicsConstraints: a.constraints.filter(c => c.constructor.name === 'PhysicsConstraintData').length };
for (const skin of (a.skins.length ? a.skins.map(s => s.name) : [null])) for (const animation of a.animations) {
  const make = (data, skin) => { const s = new spine.Skeleton(data); if (skin) s.setSkin(skin); s.setupPose(); const st = new spine.AnimationState(new spine.AnimationStateData(data)); st.setAnimation(0, animation.name, true); return [s, st]; };
  const [[sa, ta], [sb, tb]] = [make(a, skin), make(b, skin)];
  const dt = 1 / 60;
  let maxAnimDiff = 0, moved = 0;
  for (let frame = 0; frame < Number(seconds) * 60; frame++) {
    for (const [s, t] of [[sa, ta], [sb, tb]]) { t.update(dt); t.apply(s); s.update(dt); s.updateWorldTransform(spine.Physics.update); }
    for (let i = 0; i < sa.bones.length; i++) {
      const [p, q] = [sa.bones[i].appliedPose, sb.bones[i].appliedPose];
      if (sa.bones[i].data.name !== sb.bones[i].data.name) throw new Error('bone order');
      for (const k of ['a', 'b', 'c', 'd', 'worldX', 'worldY']) {
        const d = Math.abs(p[k] - q[k]); maxAnimDiff = Math.max(maxAnimDiff, d); report.boneValues++;
      }
    }
    for (let i = 0; i < sa.slots.length; i++) {
      const [x, y] = [sa.slots[i].appliedPose.attachment, sb.slots[i].appliedPose.attachment];
      if ((x?.name) !== (y?.name)) throw new Error(`attachment differs at slot ${i}`);
      if (x instanceof spine.MeshAttachment) {
        const [vx, vy] = [new Float32Array(x.worldVerticesLength), new Float32Array(y.worldVerticesLength)];
        x.computeWorldVertices(sa, sa.slots[i], 0, vx.length, vx, 0, 2); y.computeWorldVertices(sb, sb.slots[i], 0, vy.length, vy, 0, 2);
        for (let j = 0; j < vx.length; j++) { report.maxVertexDiff = Math.max(report.maxVertexDiff, Math.abs(vx[j] - vy[j])); report.vertexValues++; }
      }
    }
    report.frames++;
  }
  // Physics actually moved something: compare against a no-physics run of the original.
  const [sn, tn] = make(a, skin);
  tn.update(Number(seconds)); tn.apply(sn); sn.updateWorldTransform(spine.Physics.none);
  for (let i = 0; i < sa.bones.length; i++) moved = Math.max(moved, Math.abs(sa.bones[i].appliedPose.worldX - sn.bones[i].appliedPose.worldX), Math.abs(sa.bones[i].appliedPose.worldY - sn.bones[i].appliedPose.worldY));
  report.maxBoneDiff = Math.max(report.maxBoneDiff, maxAnimDiff);
  report.animations.push({ skin, name: animation.name, maxBoneDiff: maxAnimDiff, physicsOffsetVsNone: moved });
}
report.skins = new Set(report.animations.map(a => a.skin)).size; report.maxPhysicsOffset = Math.max(...report.animations.map(a => a.physicsOffsetVsNone)); delete report.animations; console.log(JSON.stringify(report, null, 1));
