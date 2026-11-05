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
const { Texture, TextureAtlas, AtlasAttachmentLoader, SkeletonJson, Skeleton, AnimationState, AnimationStateData, Physics } = await import(pathToFileURL(resolve(runtimePath)));
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
const data = new SkeletonJson(new AtlasAttachmentLoader(atlas)).readSkeletonData(readFileSync(exportPath, 'utf8'));
assert.equal(data.version.split('.').slice(0, 2).join('.'), '4.3');
const report = { runtime: 'Spine 4.3 external oracle', bones: data.bones.length, slots: data.slots.length, skins:data.skins.map(s=>s.name), clips: [], tolerance: 0.001, maxWorldError: 0, comparisons: 0 };
for (const animation of data.animations) {
  let maxError = 0;
  // Include the exact last frame, boundaries and backwards scrubbing.
  const times = [...Array.from({ length: 49 }, (_, i) => animation.duration * i / 48), animation.duration * 0.31, 0];
  for (const time of times) {
    const skeleton = new Skeleton(data);
    const state = new AnimationState(new AnimationStateData(data));
    skeleton.setSkin(data.skins.find(s=>s.name==='peach') ?? data.defaultSkin);
    skeleton.setupPose();
    state.setAnimation(0, animation.name, false);
    state.update(time);
    state.apply(skeleton);
    skeleton.updateWorldTransform(Physics.none);
    const ours = JSON.parse(execFileSync(cli, ['pose', source, animation.name, String(time)], { encoding: 'utf8' }));
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
  }
  report.maxWorldError = Math.max(report.maxWorldError, maxError);
  report.clips.push({ name: animation.name, samples: times.length, maxWorldError: maxError });
}
console.log(JSON.stringify(report, null, 2));
