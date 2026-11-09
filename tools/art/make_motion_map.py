"""Paint Mimi's motion map, a small image that says which parts of her may move.

Red: left pigtail. Green: right pigtail. Blue: skirt. Brighter means more movement.
Everything else stays black, so her face and smile are never bent.

Usage: make_motion_map.py <mimi.png> <motion-map.png> <check.png>
"""
import sys

import cv2
import numpy as np
from PIL import Image, ImageDraw

source, target, check = sys.argv[1:4]
image = Image.open(source).convert("RGBA")
MAP_W, MAP_H = 348, 550
# Coordinates below were traced on a 922 x 1460 view of the art.
SX, SY = MAP_W / 922, MAP_H / 1460

LEFT_HAIR = [(292, 170), (250, 200), (190, 290), (120, 375), (40, 460), (0, 530), (0, 610), (60, 650),
             (195, 660), (225, 650), (165, 610), (150, 560), (170, 500), (235, 410), (290, 330), (312, 270)]
RIGHT_HAIR = [(585, 95), (640, 190), (700, 270), (800, 320), (922, 400), (922, 480), (880, 580), (800, 615),
              (730, 590), (725, 505), (770, 560), (830, 530), (800, 440), (720, 400), (640, 370), (590, 290), (565, 180)]
SKIRT = [(345, 630), (540, 625), (645, 775), (560, 840), (440, 870), (300, 845), (180, 790)]
KEEP_STILL = [
    [(165, 530), (185, 470), (240, 415), (300, 400), (325, 640), (265, 690), (205, 640)],
    [(555, 350), (640, 380), (725, 505), (800, 660), (725, 760), (600, 600)],
    [(310, 40), (560, 40), (600, 240), (520, 350), (340, 350), (290, 240)],
]
LEFT_ROOT = (290, 180)
RIGHT_ROOT = (590, 110)
LEFT_REACH = 560.0
RIGHT_REACH = 520.0
WAIST_Y = 630.0
HEM_Y = 860.0


def polygon(points, blur: float) -> np.ndarray:
    canvas = Image.new("L", (MAP_W * 4, MAP_H * 4), 0)
    scaled = [(x * SX * 4, y * SY * 4) for x, y in points]
    ImageDraw.Draw(canvas).polygon(scaled, fill=255)
    small = np.array(canvas.resize((MAP_W, MAP_H), Image.LANCZOS)).astype(np.float32) / 255
    return cv2.GaussianBlur(small, (0, 0), blur)


ys, xs = np.mgrid[0:MAP_H, 0:MAP_W].astype(np.float32)
xs /= SX
ys /= SY


def reach(root, length: float) -> np.ndarray:
    return np.clip(np.hypot(xs - root[0], ys - root[1]) / length, 0, 1)


still = np.zeros((MAP_H, MAP_W), np.float32)
for shape in KEEP_STILL:
    still = np.maximum(still, polygon(shape, 2.5))
free = np.clip(1 - still * 1.25, 0, 1)

left = polygon(LEFT_HAIR, 3.0) * reach(LEFT_ROOT, LEFT_REACH) ** 1.25 * free
right = polygon(RIGHT_HAIR, 3.0) * reach(RIGHT_ROOT, RIGHT_REACH) ** 1.25 * free
skirt = polygon(SKIRT, 3.5) * np.clip((ys - WAIST_Y) / (HEM_Y - WAIST_Y), 0, 1) ** 1.4

motion = np.dstack([left, right, skirt])
motion = np.clip(motion * 255, 0, 255).astype(np.uint8)
Image.fromarray(motion, "RGB").save(target, optimize=True)

view_w, view_h = 553, 876
art = image.resize((view_w, view_h), Image.LANCZOS)
back = Image.new("RGBA", art.size, (24, 22, 30, 255))
back.alpha_composite(art)
big = Image.fromarray(motion, "RGB").resize((view_w, view_h), Image.BICUBIC)
overlay = Image.blend(back.convert("RGB"), big, 0.55)
pair = Image.new("RGB", (view_w * 2, view_h))
pair.paste(back.convert("RGB"), (0, 0))
pair.paste(overlay, (view_w, 0))
pair.save(check)
print("map", target, Image.open(target).size)
