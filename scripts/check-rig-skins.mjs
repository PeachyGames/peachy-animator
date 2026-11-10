// Usage: node scripts/check-rig-skins.mjs <runtime index.js> <atlas> <exported.json> <native.peachy> <peachy-cli> [samples]
// Per skin: bone activity and active-bone world transforms, ours vs runtime.
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { execFileSync } from 'node:child_process';
const [runtime, atlasPath, exportPath, nativePath, cli, samples = '5'] = process.argv.slice(2);
const spine = await import(pathToFileURL(runtime));
class T extends spine.Texture { constructor(w, h) { super({ width: w, height: h }); } setFilters() {} setWraps() {} dispose() {} }
const atlas = new spine.TextureAtlas(readFileSync(atlasPath, 'utf8'));
for (const page of atlas.pages) page.setTexture(new T(page.width, page.height));
const data = new spine.SkeletonJson(new spine.AtlasAttachmentLoader(atlas)).readSkeletonData(readFileSync(exportPath, 'utf8'));
let comparisons = 0, activity = 0, max = 0, worst = '', skipped = 0, meshValues = 0, meshMax = 0;
for (const skin of data.skins) for (const animation of data.animations) for (let i = 0; i <= Number(samples); i++) {
  const time = animation.duration * i / Number(samples);
  const s = new spine.Skeleton(data); s.setSkin(skin); s.setupPose();
  const st = new spine.AnimationState(new spine.AnimationStateData(data));
  st.setAnimation(0, animation.name, false); st.update(time); st.apply(s); s.updateWorldTransform(spine.Physics.none);
  const ours = JSON.parse(execFileSync(cli, ['pose', nativePath, animation.name, String(time), skin.name], { encoding: 'utf8', maxBuffer: 1 << 28 }));
  const draws = new Map(ours.attachments.map(a => [a.slot, a]));
  s.slots.forEach((slot, i) => {
    const att = slot.appliedPose.attachment;
    if (!(att instanceof spine.MeshAttachment) || !slot.bone.active) return;
    const v = new Float32Array(att.worldVerticesLength); att.computeWorldVertices(s, slot, 0, v.length, v, 0, 2);
    const mine = draws.get(i); if (!mine) throw new Error(`missing mesh draw ${slot.data.name}`);
    const flat = mine.vertices.flat();
    if (flat.length !== v.length) throw new Error(`mesh length ${slot.data.name}`);
    for (let j = 0; j < v.length; j++) { meshMax = Math.max(meshMax, Math.abs(flat[j] - v[j])); meshValues++; }
  });
  s.bones.forEach((bone, b) => {
    activity++;
    if (ours.bones[b].active !== bone.active) throw new Error(`${skin.name}: ${bone.data.name} activity ours ${ours.bones[b].active} vs runtime ${bone.active}`);
    if (!bone.active) { skipped++; return; }
    for (const [o, t] of [['a','a'],['b','b'],['c','c'],['d','d'],['x','worldX'],['y','worldY']]) {
      const e = Math.abs(ours.bones[b].world[o] - bone.appliedPose[t]); comparisons++;
      if (e > max) { max = e; worst = `${skin.name} ${animation.name} t=${time.toFixed(2)} ${bone.data.name}.${o}`; }
    }
  });
}
console.log(JSON.stringify({ skins: data.skins.length, activityChecks: activity, inactiveSkipped: skipped, comparisons, maxWorldError: max, worst, meshValues, meshMax }));
