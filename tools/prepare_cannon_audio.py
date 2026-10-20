"""Prepare the embedded cannon loop from the credited 48 kHz mono reference WAV.

Offline asset preparation only; requires NumPy and SciPy. See assets/audio/README.md.
Usage: python3 tools/prepare_cannon_audio.py /path/to/reference.wav
"""
import hashlib
import json
from pathlib import Path
import sys
import wave

import numpy as np
from scipy import signal

with wave.open(sys.argv[1]) as source:
    assert (source.getframerate(), source.getnchannels(), source.getsampwidth()) == (48000, 1, 2)
    raw = source.readframes(source.getnframes())
    recording = np.frombuffer(raw, dtype="<i2").astype(np.float64) / 32768.0

# The first recorded firing pass, excluding its wind-up and release.
body = recording[int(6.2 * 48000):int(7.6 * 48000)]
body = signal.sosfiltfilt(
    signal.butter(3, [40, 6500], fs=48000, btype="bandpass", output="sos"), body
)
body -= body.mean()
# Overlap the end and start over 25 ms, keeping playback continuous at wrap.
n = 1200
t = np.arange(n) / n
join = body[-n:] * np.cos(t * np.pi / 2) + body[:n] * np.sin(t * np.pi / 2)
loop = np.concatenate((body[n:-n], join))
loop *= 0.85 / np.max(np.abs(loop))
pcm = np.rint(loop * 32767).astype("<i2").tobytes()
root = Path(__file__).resolve().parents[1] / "assets" / "audio"
root.mkdir(parents=True, exist_ok=True)
(root / "a10-cannon-body.pcm").write_bytes(pcm)
report = {
    "source": "https://www.dvidshub.net/video/593024/10-gatling-gun-strafe",
    "source_pcm_sha256": hashlib.sha256(raw).hexdigest(),
    "source_start_seconds": 6.2, "source_end_seconds": 7.6,
    "sample_rate": 48000, "channels": 1, "encoding": "signed 16-bit little-endian PCM",
    "loop_samples": len(loop), "loop_seconds": len(loop) / 48000,
    "loop_sha256": hashlib.sha256(pcm).hexdigest(),
    "rms": float(np.sqrt(np.mean(loop * loop))),
    "peak": float(np.max(np.abs(loop))),
}
(root / "a10-cannon-source.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
