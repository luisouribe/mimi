# M-Mac-5 proof: stable signing and lowering the volume on Mac

Built on 2026-11-09. **Confirmed by the owner on 2026-11-09 ("Todo funcionó").**

## What changed

- **Stable signing.** `tools/mac/create_signing_certificate.sh` makes the owner's personal certificate, "Mimi Local Signing". The owner ran it. `tools/mac/env.sh` uses it for every build.
- **Lowering the volume.** A Mac has no volume for each app, so the owner chose lowering the whole output. `src-tauri/src/mac/quiet_volume.rs` plugs into the same code as Windows, which saves the level first and restores it after a crash.
- **Chime order.** On a Mac the volume drops only after the start chime, and the stop chime plays once the volume is back, so both are heard. Windows keeps its order.

## Unit tests

`unit-tests.txt`: 116 passed, 0 failed, 5 skipped, the same five skipped since `proof/M-Mac-1`. Tests can't reach the real volume, and one test checks that.

## Stable signing worked

The installed build changed from `72bc6c3e…` to `32907e55…`, with the same designated requirement, based on the certificate.

After that change, Mimi pasted into Google Chrome with no permission reset. macOS logged no `Failed to match existing code requirement` for Mimi in the 10 minutes around the test.

## From the owner's test

| Check | Result |
|---|---|
| Volume lowered while talking and returned | Confirmed by the owner |
| Output volume afterwards | 75%, the same as before |
| Saved level left behind | None, so nothing was waiting to be restored |
| Shortcut to recording | 89 ms |
| Stop speaking to text, 3.0 seconds of speech | 198 ms |
| Outcome | Pasted into Google Chrome |

## Not checked yet

- Restoring the volume after a real crash on a Mac. The shared code does this on Windows, where it's tested.
- Outputs with no volume control of their own. Mimi leaves those alone.
- The Windows build. These changes were only compiled on the Mac.
