#!/usr/bin/env python3
"""Rebuild our original Peach Bot fixture using only the Python standard library.

All shapes, colors, rig data and animation below are original Team Peachy work.
No third-party artwork or animation data is required.
"""
import json
import math
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1] / "examples" / "peach-bot"
INK = (28, 39, 47, 255)
PEACH = (247, 178, 150, 255)
LIGHT = (255, 218, 193, 255)
TEAL = (102, 194, 175, 255)
DARK = (56, 77, 85, 255)


class Canvas:
    def __init__(self, width, height):
        self.width, self.height = width, height
        self.scale = 3
        self.pixels = bytearray(width * height * self.scale**2 * 4)

    def shape(self, bounds, test, color):
        s = self.scale
        x0, y0, x1, y1 = bounds
        for y in range(max(0, int(y0*s)), min(self.height*s, math.ceil(y1*s))):
            for x in range(max(0, int(x0*s)), min(self.width*s, math.ceil(x1*s))):
                if test((x+0.5)/s, (y+0.5)/s):
                    i = (y*self.width*s+x)*4
                    self.pixels[i:i+4] = bytes(color)

    def box(self, x, y, w, h, radius, color):
        def inside(px, py):
            cx = min(max(px, x+radius), x+w-radius)
            cy = min(max(py, y+radius), y+h-radius)
            return (px-cx)**2+(py-cy)**2 <= radius**2
        self.shape((x, y, x+w, y+h), inside, color)

    def ellipse(self, x, y, w, h, color):
        self.shape((x, y, x+w, y+h), lambda px, py: ((px-x-w/2)/(w/2))**2+((py-y-h/2)/(h/2))**2 <= 1, color)

    def polygon(self, points, color):
        def inside(x, y):
            hit = False
            for a, b in zip(points, points[1:]+points[:1]):
                if (a[1] > y) != (b[1] > y) and x < (b[0]-a[0])*(y-a[1])/(b[1]-a[1])+a[0]:
                    hit = not hit
            return hit
        xs, ys = zip(*points)
        self.shape((min(xs), min(ys), max(xs), max(ys)), inside, color)

    def rgba(self):
        result = bytearray(self.width*self.height*4)
        s = self.scale
        for y in range(self.height):
            for x in range(self.width):
                samples = [self.pixels[((y*s+dy)*self.width*s+x*s+dx)*4:((y*s+dy)*self.width*s+x*s+dx)*4+4] for dy in range(s) for dx in range(s)]
                alpha = sum(p[3] for p in samples)
                i = (y*self.width+x)*4
                if alpha:
                    result[i:i+4] = bytes([round(sum(p[c]*p[3] for p in samples)/alpha) for c in range(3)]+[round(alpha/s**2)])
        return result


def png(path, width, height, pixels):
    def chunk(kind, data):
        return struct.pack(">I", len(data))+kind+data+struct.pack(">I", zlib.crc32(kind+data))
    rows = b"".join(b"\0"+pixels[y*width*4:(y+1)*width*4] for y in range(height))
    path.write_bytes(b"\x89PNG\r\n\x1a\n"+chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))+chunk(b"IDAT", zlib.compress(rows))+chunk(b"IEND", b""))


def shell(c, x, y, w, h, radius, color):
    c.box(x, y, w, h, radius, INK)
    c.box(x+3, y+3, w-6, h-6, radius-3, color)


def artwork():
    parts = {}
    c = Canvas(140, 148)
    shell(c, 5, 5, 130, 138, 38, PEACH)
    c.box(17, 17, 106, 112, 28, LIGHT)
    shell(c, 34, 39, 72, 65, 20, DARK)
    c.ellipse(54, 57, 32, 30, PEACH)
    c.polygon([(68, 58), (73, 42), (90, 45), (83, 56)], TEAL)
    c.box(43, 115, 54, 5, 2, PEACH)
    parts["torso"] = c
    c = Canvas(142, 134)
    c.box(66, 5, 7, 25, 3, INK)
    c.polygon([(68, 13), (74, 1), (100, 4), (88, 17)], INK)
    c.polygon([(73, 12), (77, 5), (94, 7), (86, 13)], TEAL)
    shell(c, 6, 25, 130, 103, 35, PEACH)
    c.box(16, 35, 110, 80, 28, LIGHT)
    shell(c, 23, 48, 96, 49, 18, DARK)
    c.box(41, 62, 11, 20, 5, TEAL)
    c.box(90, 62, 11, 20, 5, TEAL)
    c.box(61, 101, 20, 4, 2, PEACH)
    parts["head"] = c
    for name, w, h, fill in [("upper-arm", 82, 42, TEAL), ("forearm", 78, 40, LIGHT), ("thigh", 100, 49, DARK), ("shin", 88, 45, TEAL)]:
        c = Canvas(w, h)
        shell(c, 3, 3, w-6, h-6, (h-6)/2, fill)
        c.ellipse(10, h/2-8, 16, 16, INK)
        c.ellipse(13, h/2-5, 10, 10, PEACH)
        c.box(31, 10, w-49, 4, 2, (255, 235, 209, 255))
        parts[name] = c
    c = Canvas(54, 56)
    shell(c, 4, 9, 43, 43, 17, PEACH)
    c.box(29, 3, 12, 28, 6, INK)
    c.box(32, 6, 6, 21, 3, LIGHT)
    c.box(12, 18, 22, 21, 9, LIGHT)
    parts["hand"] = c
    c = Canvas(81, 45)
    shell(c, 4, 4, 73, 36, 14, PEACH)
    c.box(8, 31, 65, 7, 3, INK)
    c.box(13, 10, 47, 9, 4, LIGHT)
    parts["foot"] = c
    for original in ["torso", "head"]:
        source = parts[original]
        variant = Canvas(source.width, source.height)
        palette = {PEACH: TEAL, LIGHT: (199, 236, 220, 255), TEAL: PEACH}
        for i in range(0, len(source.pixels), 4):
            pixel = tuple(source.pixels[i:i+4])
            variant.pixels[i:i+4] = bytes(palette.get(pixel, pixel))
        parts[original+"-mint"] = variant
    return parts


def transform(x=0, y=0, rotation=0):
    return dict(x=x, y=y, rotation=rotation, scale_x=1, scale_y=1)


def main():
    (ROOT/"images").mkdir(parents=True, exist_ok=True)
    parts = artwork()
    atlas_pixels = bytearray(512*512*4)
    atlas = ["peach-bot.png", "size: 512,512", "filter: Linear,Linear", "pma: false"]
    x = y = 2
    row = 0
    for name, c in parts.items():
        pixels = c.rgba()
        png(ROOT/"images"/(name+".png"), c.width, c.height, pixels)
        if x+c.width+2 > 512:
            x = 2
            y += row+4
            row = 0
        for iy in range(c.height):
            start = ((y+iy)*512+x)*4
            atlas_pixels[start:start+c.width*4] = pixels[iy*c.width*4:(iy+1)*c.width*4]
        atlas.extend([name, f"  bounds: {x},{y},{c.width},{c.height}"])
        x += c.width+4
        row = max(row, c.height)
    png(ROOT/"peach-bot.png", 512, 512, atlas_pixels)
    (ROOT/"peach-bot.atlas").write_text("\n".join(atlas)+"\n")
    bones = []
    def bone(name, parent, x, y, angle=0, length=0):
        bones.append(dict(name=name, parent=parent, length=length, setup=transform(x, y, angle)))
        return len(bones)-1
    root = bone("root", None, 0, 0)
    body = bone("body", root, 0, 125, 0, 95)
    head = bone("head", body, 0, 118, 0, 64)
    arm_l = bone("arm.L", body, -70, 48, -128, 64)
    fore_l = bone("forearm.L", arm_l, 64, 0, 30, 60)
    hand_l = bone("hand.L", fore_l, 60, 0, 0, 28)
    arm_r = bone("arm.R", body, 70, 48, 38, 64)
    fore_r = bone("forearm.R", arm_r, 64, 0, 45, 60)
    hand_r = bone("hand.R", fore_r, 60, 0, 0, 28)
    leg_l = bone("leg.L", body, -34, -61, -102, 82)
    shin_l = bone("shin.L", leg_l, 82, 0, 15, 72)
    foot_l = bone("foot.L", shin_l, 72, 0, 87, 36)
    leg_r = bone("leg.R", body, 34, -61, -78, 82)
    shin_r = bone("shin.R", leg_r, 82, 0, -15, 72)
    foot_r = bone("foot.R", shin_r, 72, 0, 93, 36)
    slots, skin = [], {}
    def region(name, b, part, x=0, y=0, rotation=0):
        slots.append(dict(name=name, bone=b, attachment=name))
        c = parts[part]
        skin[name] = {name: dict(type="region", path=part, width=c.width, height=c.height, transform=transform(x, y, rotation))}
    for label, leg, shin, foot in [("L", leg_l, shin_l, foot_l), ("R", leg_r, shin_r, foot_r)]:
        region("thigh."+label, leg, "thigh", 38)
        region("shin."+label, shin, "shin", 33)
        region("foot."+label, foot, "foot", 12, -5)
    region("upper-arm.L", arm_l, "upper-arm", 29)
    region("forearm.L", fore_l, "forearm", 27)
    region("hand.L", hand_l, "hand", 10, rotation=-90)
    region("body", body, "torso", 0, 3)
    region("head", head, "head", 0, 12)
    region("upper-arm.R", arm_r, "upper-arm", 29)
    region("forearm.R", fore_r, "forearm", 27)
    region("hand.R", hand_r, "hand", 10, rotation=-90)
    def track(b, prop, values, curve="linear"):
        return dict(bone=b, property=prop, keys=[dict(time=t, value=v, curve=curve) for t,v in values])
    wave = [track(hand_r, "rotation", [(0,-15),(.3,25),(.6,-18),(.9,25),(1.2,-18),(1.5,25),(1.8,-15),(2.4,-15)]),
            track(fore_r,"rotation",[(0,0),(.6,12),(1.2,0),(1.8,12),(2.4,0)]),
            track(body,"y",[(0,0),(.6,5),(1.2,0),(1.8,5),(2.4,0)], {"bezier":[.42,0,.58,1]}),
            track(head,"rotation",[(0,-4),(1.2,4),(2.4,-4)], {"bezier":[.42,0,.58,1]})]
    idle = [track(body,"y",[(0,0),(1,4),(2,0)], {"bezier":[.42,0,.58,1]}), track(head,"rotation",[(0,-2),(1,2),(2,-2)])]
    mint = {}
    peach = {}
    for slot, texture in [("body", "torso-mint"), ("head", "head-mint")]:
        mint[slot] = {slot: {**skin[slot][slot], "path": texture, "name": slot+".mint"}}
        peach[slot] = {slot: {**skin[slot][slot], "name": slot+".peach"}}
    # Skin placeholders must not collide with direct attachments in default.
    shared = {slot: entries for slot, entries in skin.items() if slot not in peach}
    doc = dict(format=2, name="Peach Bot", fps=30, bones=bones, slots=slots, skins={"default":shared,"peach":peach,"mint":mint}, animations=[dict(name="wave",duration=2.4,tracks=wave),dict(name="idle",duration=2,tracks=idle)])
    (ROOT/"peach-bot.peachy").write_text(json.dumps(doc, indent=2)+"\n")
    contract = json.loads(json.dumps(doc))
    contract["name"] = "Transform Contract"
    contract["bones"][0]["setup"].update(rotation=37, scale_x=1.3, scale_y=.8)
    def contract_keys(a, b, curve):
        return [dict(time=0, value=a, curve=curve), dict(time=1, value=b, curve="linear"), dict(time=2, value=a, curve="linear")]
    contract["animations"] = [dict(name="contract", duration=2, tracks=[
        dict(bone=1, property="x", keys=contract_keys(-10, 28, {"bezier":[.2,.3,.8,1]})),
        dict(bone=1, property="y", keys=contract_keys(0, 20, "stepped")),
        dict(bone=6, property="scaleX", keys=contract_keys(1, 1.4, {"bezier":[.42,0,.58,1]})),
        dict(bone=6, property="scaleY", keys=contract_keys(1, .6, "linear")),
        dict(bone=2, property="rotation", keys=contract_keys(0, 400, "linear")),
    ])]
    (ROOT/"contract-cases.peachy").write_text(json.dumps(contract, indent=2)+"\n")
    # A separate mesh fixture keeps the region fixture as a regression reference.
    mesh_doc = json.loads(json.dumps(doc))
    mesh_doc["name"] = "Peach Bot Mesh Lab"
    import math
    for skin_name, slot_name in [("default", "hand.R"), ("peach", "body"), ("mint", "body")]:
        r = mesh_doc["skins"][skin_name][slot_name][slot_name]
        w, h = r["width"]/2, r["height"]/2
        t = r["transform"]
        angle = math.radians(t["rotation"])
        def local(x, y):
            x *= t["scale_x"]; y *= t["scale_y"]
            return [t["x"]+x*math.cos(angle)-y*math.sin(angle), t["y"]+x*math.sin(angle)+y*math.cos(angle)]
        vertices = [local(-w,-h),local(w,-h),local(w,h),local(-w,h),local(0,0)]
        # Setup deformation: the UVs stay fixed while the center moves.
        vertices[4][0] += 8
        mesh = dict(type="mesh",path=r["path"],width=r["width"],height=r["height"],vertices=vertices,
                    uvs=[[0,1],[1,1],[1,0],[0,0],[.5,.5]],triangles=[0,1,4,1,2,4,2,3,4,3,0,4],
                    hull=4,edges=[0,2,2,4,4,6,6,0])
        if "name" in r: mesh["name"]=r["name"]
        mesh_doc["skins"][skin_name][slot_name][slot_name]=mesh
    (ROOT/"mesh-bot.peachy").write_text(json.dumps(mesh_doc,indent=2)+"\n")
    mesh_contract = json.loads(json.dumps(mesh_doc))
    mesh_contract["name"] = "Mesh Transform Contract"
    mesh_contract["bones"][0]["setup"].update(rotation=37,scale_x=-1.3,scale_y=.8)
    mesh_contract["animations"] = contract["animations"]
    (ROOT/"mesh-contract.peachy").write_text(json.dumps(mesh_contract,indent=2)+"\n")
    print(f"Wrote original fixture: {len(bones)} bones, {len(slots)} slots, 2 animations, {len(parts)} textures")


if __name__ == "__main__":
    main()
