# Blender → After Effects OSS camera track exporter
#
# 使い方 (Blender 3.x/4.x):
#   1. モーショントラッキングで解決したカメラをシーンのアクティブカメラに設定
#   2. このファイルをBlenderの Text Editor に貼り付けて Open
#   3. 下の OUTPUT_PATH を書き出し先に変更(任意)
#   4. Run Script ▶ → 同フォルダに JSON が保存される
#   5. After Effects OSS の File > Import > Blender Camera Track (.json)
#
# OSS-only workflow: bpy はBlender同梱、追加アドオン不要。

import bpy
import json
import math
from mathutils import Vector

OUTPUT_PATH = "//camera_track.json"   # "//" = .blendと同じフォルダ
UP_AXIS = "Z"                          # BlenderはZ-up

scene = bpy.context.scene
cam_obj = scene.camera
if cam_obj is None:
    raise RuntimeError("アクティブカメラがありません (Scene > Camera)")

fps = scene.render.fps
frame_start = scene.frame_start
frame_end = scene.frame_end

deg = lambda r: round(math.degrees(r), 4)

frames = []
for f in range(frame_start, frame_end + 1):
    scene.frame_set(f)
    # アニメーションやコンストレイントを含むワールド行列からベイク
    mw = cam_obj.matrix_world
    loc = mw.to_translation()
    eul = mw.to_euler('XYZ')
    frames.append({
        "frame": f - frame_start,           # 0起点に正規化
        "pos": [round(loc.x, 5), round(loc.y, 5), round(loc.z, 5)],
        "rot_deg": [deg(eul.x), deg(eul.y), deg(eul.z)],
    })

# FOV: センサー幅と焦点距離から水平画角を計算
cam_data = cam_obj.data
sensor_w = cam_data.sensor_width
lens = max(cam_data.lens, 0.01)
import math as _m
fov_h = _m.degrees(2 * _m.atan(sensor_w / (2 * lens)))

out = {
    "generator": "blender_camera_export.py",
    "source_camera": cam_obj.name,
    "fps": fps,
    "up_axis": UP_AXIS,
    "scale": 1.0,          # 必要ならここを変更 (例: 0.01 でcm→m)
    "fov": round(fov_h, 3),
    "range": [frame_start, frame_end],
    "frames": frames,
}

path = bpy.path.abspath(OUTPUT_PATH)
with open(path, "w", encoding="utf-8") as fp:
    json.dump(out, fp, indent=1)

print("Camera track written:", path, "frames:", len(frames))
