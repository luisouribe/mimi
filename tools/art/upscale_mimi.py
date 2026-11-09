"""Upscale Mimi's repaired artwork four times and cut her out on transparency.

Usage: upscale_mimi.py <model.onnx> <repaired.png> <hair mask> <erase mask> <output folder>

The Real-ESRGAN anime model runs inside ONNX Runtime, in small overlapping tiles.
"""
import sys
from pathlib import Path

import cv2
import numpy as np
import onnxruntime as ort
from PIL import Image
from rembg import new_session, remove

MODEL, REPAIRED, HAIR, ERASE, OUTPUT = sys.argv[1:6]
OUT = Path(OUTPUT)
OUT.mkdir(parents=True, exist_ok=True)
BOX = (470, 20, 985, 768)
SCALE = 4
FINAL_HEIGHT = 2200

session = ort.InferenceSession(MODEL, providers=["CPUExecutionProvider"])
meta = session.get_inputs()[0]
print("model input", meta.name, meta.shape, meta.type, flush=True)
fixed = meta.shape[2] if isinstance(meta.shape[2], int) else None
TILE = fixed or 192
PAD = 16
CORE = TILE - 2 * PAD


def upscale(rgb: np.ndarray) -> np.ndarray:
    height, width, _ = rgb.shape
    padded = cv2.copyMakeBorder(rgb, PAD, PAD + TILE, PAD, PAD + TILE, cv2.BORDER_REFLECT_101)
    result = np.zeros((height * SCALE, width * SCALE, 3), np.float32)
    rows = range(0, height, CORE)
    columns = range(0, width, CORE)
    total = len(rows) * len(columns)
    done = 0
    for top in rows:
        for left in columns:
            tile = padded[top:top + TILE, left:left + TILE].astype(np.float32) / 255
            batch = tile.transpose(2, 0, 1)[None]
            output = session.run(None, {meta.name: batch})[0][0].transpose(1, 2, 0)
            core = output[PAD * SCALE:(PAD + CORE) * SCALE, PAD * SCALE:(PAD + CORE) * SCALE]
            bottom = min(top * SCALE + core.shape[0], result.shape[0])
            right = min(left * SCALE + core.shape[1], result.shape[1])
            result[top * SCALE:bottom, left * SCALE:right] = core[:bottom - top * SCALE, :right - left * SCALE]
            done += 1
            print(f"tile {done}/{total}", flush=True)
    return np.clip(result * 255, 0, 255).astype(np.uint8)


source = np.array(Image.open(REPAIRED).convert("RGB").crop(BOX))
big_path = OUT / "mimi-upscaled-background.png"
if big_path.exists():
    big = np.array(Image.open(big_path).convert("RGB"))
else:
    big = upscale(source)
    Image.fromarray(big).save(big_path)
size = (big.shape[1], big.shape[0])


def layer(path: str) -> np.ndarray:
    mask = Image.open(path).convert("L").crop(BOX).resize(size, Image.BICUBIC)
    return np.array(mask).astype(np.float32) / 255


cut = remove(Image.fromarray(big), session=new_session("isnet-anime"), post_process_mask=False)
alpha = np.array(cut.getchannel("A")).astype(np.float32) / 255
alpha = np.maximum(alpha, layer(HAIR)) * (1 - layer(ERASE))

# Tighten the soft edge so no grey background halo survives.
alpha = cv2.GaussianBlur(alpha, (0, 0), 1.2)
alpha = cv2.erode(alpha, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
alpha = np.clip((alpha - 0.30) / 0.45, 0, 1)

# Replace edge colours with colours from just inside her outline.
solid = (alpha > 0.97).astype(np.float32)
colour = big.astype(np.float32)
weight = cv2.GaussianBlur(solid, (0, 0), 5) + 1e-4
spread = cv2.GaussianBlur(colour * solid[..., None], (0, 0), 5) / weight[..., None]
use_spread = (1 - solid)[..., None]
colour = colour * (1 - use_spread) + spread * use_spread

rgba = np.dstack([np.clip(colour, 0, 255).astype(np.uint8), (alpha * 255).astype(np.uint8)])
master = Image.fromarray(rgba, "RGBA")
master = master.crop(master.getchannel("A").getbbox())
master.save(OUT / "mimi-master.png", optimize=True)
final = master.resize((round(master.width * FINAL_HEIGHT / master.height), FINAL_HEIGHT), Image.LANCZOS)
final.save(OUT / "mimi.png", optimize=True)
print("master", master.size, "final", final.size, flush=True)
