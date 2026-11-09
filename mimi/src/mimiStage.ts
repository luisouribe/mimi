// Mimi's living idle animation.
// The motion map says which parts may move (red: left pigtail, green: right pigtail,
// blue: skirt). Her face is black on the map, so her smile is never bent.

export type MimiPhase = "idle" | "listening" | "working";

const VERTEX = `#version 300 es
in vec2 position;
out vec2 vUv;
void main() {
  vUv = vec2(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
  gl_Position = vec4(position, 0.0, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 color;
uniform sampler2D art;
uniform sampler2D motion;
uniform float time;
uniform float phase;
uniform float energy;
uniform float level;
uniform float glow;
const vec2 padStart = vec2(60.0 / 585.0, 24.0 / 766.0);
const vec2 padEnd = vec2(60.0 / 585.0, 6.0 / 766.0);
const vec2 aspect = vec2(0.632, 1.0);
const vec2 feet = vec2(0.46, 0.985);
void main() {
  vec2 uv = (vUv - padStart) / (1.0 - padStart - padEnd);
  float breath = sin(time * 1.35);
  float lean = sin(time * 0.62 + 0.7) * 0.0032 * (1.0 + 0.6 * energy);
  vec2 p = (uv - feet) * aspect;
  float c = cos(lean);
  float s = sin(lean);
  p = vec2(c * p.x + s * p.y, -s * p.x + c * p.y);
  p.y /= 1.0 + 0.0045 * breath + 0.0045 * level * energy;
  uv = p / aspect + feet;
  uv.y += (0.5 + 0.5 * breath) * 0.007;
  vec3 m = texture(motion, uv).rgb;
  // The pigtails only float slightly. There is no wave along them, so they never bend.
  vec2 move = vec2(0.0);
  move.x += m.r * 0.006 * sin(phase * 1.1);
  move.y += m.r * 0.002 * sin(phase * 1.1 + 1.2);
  move.x += m.g * 0.006 * sin(phase * 0.95 + 2.1);
  move.y += m.g * 0.002 * sin(phase * 0.95 + 3.3);
  move.x += m.b * 0.0065 * (1.0 + 0.7 * energy) * sin(phase * 2.0 - m.b * 2.0);
  uv -= move;
  vec4 pixel = texture(art, uv);
  if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) pixel = vec4(0.0);
  if (pixel.a > 0.01) {
    vec3 straight = pixel.rgb / pixel.a;
    float cyan = smoothstep(0.04, 0.2, straight.b - straight.r) * smoothstep(-0.06, 0.06, straight.g - straight.r);
    pixel.rgb += vec3(0.35, 0.8, 1.0) * cyan * glow * pixel.a * 0.4;
  }
  color = pixel;
}`;

// The motion is slow, so 30 frames per second looks the same and costs far less.
const FRAME_MS = 1000 / 30 - 2;

export interface MimiStage {
  setActive(active: boolean): void;
  stop(): void;
}

function approach(value: number, target: number, dt: number, seconds: number) {
  return value + (target - value) * (1 - Math.exp(-dt / seconds));
}

/** Starts the animation. Throws if it cannot run. */
export async function startMimiStage(
  canvas: HTMLCanvasElement,
  artUrl: string,
  mapUrl: string,
  getPhase: () => MimiPhase,
  getVoice: () => number,
): Promise<MimiStage> {
  const gl = canvas.getContext("webgl2", { alpha: true, premultipliedAlpha: true, antialias: false });
  if (!gl) throw new Error("WebGL2 is unavailable");

  const load = async (url: string, options: ImageBitmapOptions) =>
    createImageBitmap(await (await fetch(url)).blob(), options);
  const [art, map] = await Promise.all([
    load(artUrl, { premultiplyAlpha: "premultiply", colorSpaceConversion: "none" }),
    load(mapUrl, { premultiplyAlpha: "none", colorSpaceConversion: "none" }),
  ]);

  const compile = (type: number, source: string) => {
    const shader = gl.createShader(type)!;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader) ?? "shader");
    return shader;
  };
  const program = gl.createProgram()!;
  gl.attachShader(program, compile(gl.VERTEX_SHADER, VERTEX));
  gl.attachShader(program, compile(gl.FRAGMENT_SHADER, FRAGMENT));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program) ?? "program");
  gl.useProgram(program);

  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
  const position = gl.getAttribLocation(program, "position");
  gl.enableVertexAttribArray(position);
  gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);

  const addTexture = (unit: number, name: string, image: ImageBitmap, mipmaps: boolean) => {
    gl.activeTexture(gl.TEXTURE0 + unit);
    gl.bindTexture(gl.TEXTURE_2D, gl.createTexture());
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, mipmaps ? gl.LINEAR_MIPMAP_LINEAR : gl.LINEAR);
    if (mipmaps) gl.generateMipmap(gl.TEXTURE_2D);
    gl.uniform1i(gl.getUniformLocation(program, name), unit);
  };
  addTexture(0, "art", art, true);
  addTexture(1, "motion", map, false);
  art.close();
  map.close();

  const uniform = (name: string) => gl.getUniformLocation(program, name);
  const uniforms = {
    time: uniform("time"),
    phase: uniform("phase"),
    energy: uniform("energy"),
    level: uniform("level"),
    glow: uniform("glow"),
  };

  let frameId = 0;
  let active = true;
  let last = performance.now();
  let time = 0;
  let phase = 0;
  let energy = 0;
  let level = 0;
  let glow = 0.12;

  const draw = (dt: number) => {
    time += dt;
    const current = getPhase();
    const listening = current === "listening";
    energy = approach(energy, listening ? 1 : 0, dt, 0.25);
    phase += dt * (1 + 0.45 * energy);
    const voice = getVoice();
    level = approach(level, listening ? voice : 0, dt, 0.06);
    glow = approach(glow, listening ? 0.3 + 0.7 * voice : current === "working" ? 0.6 : 0.12, dt, 0.09);

    const scale = Math.min(window.devicePixelRatio || 1, 2);
    const width = Math.round(canvas.clientWidth * scale);
    const height = Math.round(canvas.clientHeight * scale);
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
      gl.viewport(0, 0, width, height);
    }
    gl.uniform1f(uniforms.time, time);
    gl.uniform1f(uniforms.phase, phase);
    gl.uniform1f(uniforms.energy, energy);
    gl.uniform1f(uniforms.level, level);
    gl.uniform1f(uniforms.glow, glow);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  };

  const frame = (now: number) => {
    frameId = requestAnimationFrame(frame);
    if (now - last < FRAME_MS) return;
    const dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    draw(dt);
  };

  const update = () => {
    cancelAnimationFrame(frameId);
    if (active && !document.hidden) {
      last = performance.now();
      frameId = requestAnimationFrame(frame);
    }
  };
  document.addEventListener("visibilitychange", update);
  // Draw once right away, so she is on screen even if the animation starts paused.
  draw(0);
  update();

  return {
    setActive(next) {
      if (next === active) return;
      active = next;
      update();
    },
    stop() {
      cancelAnimationFrame(frameId);
      document.removeEventListener("visibilitychange", update);
    },
  };
}
