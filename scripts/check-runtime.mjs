#!/usr/bin/env node
/** Black-box interoperability check. Supply a separately licensed Spine runtime.
 * No official runtime implementation is copied, bundled or vendored here.
 * Usage: node scripts/check-runtime.mjs /path/to/spine-core/dist/index.js
 */
import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const runtimePath = process.argv[2];
if (!runtimePath) throw new Error('Supply an absolute path to your separately licensed Spine 4.3 runtime module.');
const { Texture, TextureAtlas, AtlasAttachmentLoader, SkeletonJson, Skeleton, AnimationState, AnimationStateData, Physics, MeshAttachment, RegionAttachment } = await import(pathToFileURL(resolve(runtimePath)));
class OracleTexture extends Texture {
  constructor(width, height) { super({ width, height }); }
  setFilters() {}
  setWraps() {}
  dispose() {}
}
const cli = resolve(root, 'target/debug/peachy-cli');
const source = process.argv[5] ? resolve(process.argv[5]) : resolve(root, 'examples/peach-bot/peach-bot.peachy');
const exportPath = process.argv[3] ? resolve(process.argv[3]) : resolve(root, '.local/peach-bot.json');
const atlasPath = process.argv[4] ? resolve(process.argv[4]) : resolve(root, 'examples/peach-bot/peach-bot.atlas');
if (!existsSync(exportPath)) execFileSync(cli, ['export', source, exportPath]);
const atlas = new TextureAtlas(readFileSync(atlasPath, 'utf8'));
for (const page of atlas.pages) {
  const imagePath=resolve(dirname(atlasPath),page.name);
  assert.ok(existsSync(imagePath), `Missing atlas page ${page.name}`);
  const png=readFileSync(imagePath);
  assert.equal(png.subarray(0,8).toString('hex'),'89504e470d0a1a0a','Oracle accepts PNG atlas pages');
  const width=png.readUInt32BE(16),height=png.readUInt32BE(20);
  assert.equal(page.width,width,'Atlas width matches the actual PNG');
  assert.equal(page.height,height,'Atlas height matches the actual PNG');
  page.setTexture(new OracleTexture(width,height));
}
const sourceProject = JSON.parse(readFileSync(source, 'utf8'));
const data = new SkeletonJson(new AtlasAttachmentLoader(atlas)).readSkeletonData(readFileSync(exportPath, 'utf8'));
assert.equal(data.version.split('.').slice(0, 2).join('.'), '4.3');
assert.equal(data.animations.length, sourceProject.animations.length, 'Every native clip is exported, including empty clips');
assert.equal(data.constraints.length, (sourceProject.constraints ?? []).length, 'Every native constraint retains its ordered identity');
const report = { regionComparisons: 0, maxRegionError: 0, meshComparisons: 0, weightedMeshComparisons: 0, maxMeshError: 0, uvComparisons: 0, maxUVError: 0, runtime: 'Spine 4.3 external oracle', bones: data.bones.length, slots: data.slots.length, skins:data.skins.map(s=>s.name), clips: [], tolerance: 0.001, maxWorldError: 0, comparisons: 0, ikPoseComparisons: 0, maxIKMixError: 0, ikMixTolerance: 1e-6 };
for (const animation of data.animations) {
  const native = sourceProject.animations.find(clip => clip.name === animation.name);
  assert.ok(native, 'Exported clip has a native source');
  const nativeEnd = sourceProject.format >= 3
    ? [...native.tracks, ...(native.ik_tracks ?? [])].reduce((end, track) => track.keys.reduce((end, key) => Math.max(end, Math.fround(key.time)), end), 0)
    : Math.fround(native.duration);
  assert.equal(animation.duration, nativeEnd, `${animation.name}: runtime end matches native end`);
  let maxError = 0;
  // Include the exact last frame, held poses beyond the end, and backwards scrubbing.
  const times = [...Array.from({ length: 49 }, (_, i) => animation.duration * i / 48), animation.duration * 0.31, animation.duration + 1, 0].map(Math.fround);
  for (const skin of data.skins.length ? data.skins : [null]) {
    for (const time of times) {
      const skeleton = new Skeleton(data);
      const state = new AnimationState(new AnimationStateData(data));
      if (skin) skeleton.setSkin(skin);
      skeleton.setupPose();
      state.setAnimation(0, animation.name, false);
      state.update(time);
      state.apply(skeleton);
      skeleton.updateWorldTransform(Physics.none);
      const ours = JSON.parse(execFileSync(cli, ['pose', source, animation.name, String(time), skin?.name ?? 'default'], { encoding: 'utf8' }));
      assert.ok(Array.isArray(ours.ikPoses), 'CLI exposes the sampled IK settings independently of bone locals');
      assert.equal(ours.ikPoses.length, skeleton.constraints.length, 'IK pose group count');
      for (let i = 0; i < skeleton.constraints.length; i++) {
        const constraint = skeleton.constraints[i];
        const actual = ours.ikPoses[i];
        const nativeConstraint = sourceProject.constraints[i];
        assert.equal(actual.constraint, i, 'Typed constraint index');
        assert.equal(actual.name, nativeConstraint.name, 'Native IK group identity');
        assert.equal(actual.name, constraint.data.name, 'Runtime IK group identity');
        const error = Math.abs(actual.mix - constraint.pose.mix);
        assert.ok(Number.isFinite(error) && error <= report.ikMixTolerance,
          `${animation.name} t=${time} IK=${actual.name} Mix error=${error}`);
        assert.equal(actual.bendPositive, constraint.pose.bendDirection > 0, 'Discrete IK bend');
        report.ikPoseComparisons++;
        report.maxIKMixError = Math.max(report.maxIKMixError, error);
      }
      for (let i = 0; i < skeleton.bones.length; i++) {
        const official = skeleton.bones[i].appliedPose;
        const actual = ours.bones[i].world;
        assert.equal(ours.bones[i].name, skeleton.bones[i].data.name);
        for (const [ourKey, theirKey] of [['a','a'],['b','b'],['c','c'],['d','d'],['x','worldX'],['y','worldY']]) {
          const error = Math.abs(actual[ourKey] - official[theirKey]);
          assert.ok(Number.isFinite(error), `${animation.name} ${time} ${theirKey} is nonfinite`);
          assert.ok(error <= report.tolerance, `${animation.name} t=${time} bone=${ours.bones[i].name} ${ourKey}: ours ${actual[ourKey]} vs Spine ${official[theirKey]} (error ${error})`);
          maxError = Math.max(maxError, error);
          report.comparisons++;
        }
      }
      const actualDraws = new Map(ours.attachments.map(a=>[a.slot,a]));
      for (const draw of ours.attachments) {
        const key=draw.attachment;
        const native=sourceProject.skins[key.skin][key.slot][key.placeholder];
        assert.equal(skeleton.slots[draw.slot].data.name,key.slot,'Slot identity');
        if (native.type==='mesh') assert.ok(skeleton.slots[draw.slot].appliedPose.attachment instanceof MeshAttachment,'Native mesh remains a mesh in the external runtime');
        if (native.vertices?.mode === 'weighted') {
          assert.ok(skeleton.slots[draw.slot].appliedPose.attachment.bones?.length > 0, 'Native weighted mesh remains weighted in the external runtime');
        }
        if (native.type==='region' || sourceProject.format===1) assert.ok(skeleton.slots[draw.slot].appliedPose.attachment instanceof RegionAttachment,'Native region remains a region in the external runtime');
      }
      for (let slotIndex=0;slotIndex<skeleton.slots.length;slotIndex++) {
        const slot=skeleton.slots[slotIndex];
        const attachment=slot.appliedPose.attachment;
        if (attachment instanceof RegionAttachment) {
          const actual=actualDraws.get(slotIndex);
          assert.ok(actual,`Missing region at slot ${slotIndex}`);
          const vertices=new Float32Array(8);
          attachment.computeWorldVertices(slot,attachment.getOffsets(slot.appliedPose),vertices,0,2);
          const uvs=attachment.sequence.getUVs(0),region=attachment.sequence.regions[0];
          assert.ok(region);assert.equal(region.degrees ?? 0,0,'Unrotated region fixture');
          for (let i=0;i<4;i++) {
            const local=[(uvs[i*2]-region.u)/(region.u2-region.u),(uvs[i*2+1]-region.v)/(region.v2-region.v)];
            const ours=actual.uvs.findIndex(uv=>Math.abs(uv[0]-local[0])<1e-6 && Math.abs(uv[1]-local[1])<1e-6);
            assert.ok(ours>=0,'Region corner identified by UV');
            for (let axis=0;axis<2;axis++) {
              const error=Math.abs(actual.vertices[ours][axis]-vertices[i*2+axis]);
              assert.ok(Number.isFinite(error) && error<=report.tolerance,`${animation.name} skin=${skin?.name} region=${slotIndex} error=${error}`);
              report.regionComparisons++;report.maxRegionError=Math.max(report.maxRegionError,error);
            }
          }
        }
        if (!(attachment instanceof MeshAttachment)) continue;
        const actual=actualDraws.get(slotIndex);
        assert.ok(actual,`Missing mesh at slot ${slotIndex}`);
        const native=sourceProject.skins[actual.attachment.skin][actual.attachment.slot][actual.attachment.placeholder];
        const weighted=native.vertices?.mode === 'weighted';
        assert.equal(actual.image,attachment.path);
        assert.deepEqual(actual.triangles,Array.from(attachment.triangles),'Triangle order');
        assert.equal(actual.hull*2,attachment.hullLength,'Hull coordinate count');
        const vertices=new Float32Array(attachment.worldVerticesLength);
        attachment.computeWorldVertices(skeleton,slot,0,vertices.length,vertices,0,2);
        const oursVertices=actual.vertices.flat();
        assert.equal(oursVertices.length,vertices.length);
        for (let i=0;i<vertices.length;i++) {
          const error=Math.abs(oursVertices[i]-vertices[i]);
          assert.ok(Number.isFinite(error) && error<=report.tolerance,`${animation.name} skin=${skin?.name} t=${time} mesh=${slotIndex} coordinate=${i} error=${error}`);
          report.meshComparisons++; report.maxMeshError=Math.max(report.maxMeshError,error);
          if (weighted) report.weightedMeshComparisons++;
        }
        const localUVs=actual.uvs.flat();
        const packedUVs=attachment.sequence.getUVs(0);
        const region=attachment.sequence.regions[0];
        assert.ok(region,'Mesh texture region'); assert.equal(region.degrees ?? 0,0,'Fixture atlas is unrotated');
        assert.equal(localUVs.length,attachment.regionUVs.length);
        for (let i=0;i<localUVs.length;i++) {
          const error=Math.abs(localUVs[i]-attachment.regionUVs[i]);
          assert.ok(Number.isFinite(error) && error<=1e-6,'Local UV agreement');
          const expected=i%2===0 ? region.u+localUVs[i]*(region.u2-region.u) : region.v+localUVs[i]*(region.v2-region.v);
          const packedError=Math.abs(expected-packedUVs[i]);
          assert.ok(Number.isFinite(packedError) && packedError<=1e-6,'Packed UV agreement');
          report.uvComparisons+=2; report.maxUVError=Math.max(report.maxUVError,error,packedError);
        }
      }

    }
  }
  report.maxWorldError = Math.max(report.maxWorldError, maxError);
  report.clips.push({ name: animation.name, nativeEnd, runtimeEnd: animation.duration, samplesPerSkin: times.length, skins: data.skins.map(s=>s.name), maxWorldError: maxError });
}
console.log(JSON.stringify(report, null, 2));
