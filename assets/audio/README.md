# Cannon recording credit

`a10-cannon-body.pcm` is an edited audio excerpt from **A-10 gatling gun strafe**,
U.S. Air Force / 355th Wing, recorded March 21, 2018, by Senior Airman Betty
Chevalier (now credited by DVIDS as TSgt Betty Chevalier).

- Source: https://www.dvidshub.net/video/593024/10-gatling-gun-strafe
- DVIDS video 593024; VIRIN 180321-F-WQ860-001; file DOD_105476732.
- The source page marks the work **PUBLIC DOMAIN**.
- Use notice: https://www.dvidshub.net/about/copyright
- No endorsement by the U.S. Government or military is implied.

The excerpt uses 6.2–7.6 seconds of the first onboard firing pass. It is bandpass
filtered at 40–6,500 Hz, DC centered, crossfaded over 25 ms for looping, and peak
normalized to 0.85. See `a10-cannon-source.json` for hashes and measurements and
`tools/prepare_cannon_audio.py` for reproducible processing from a decoded mono
48 kHz, 16-bit WAV. The original video's ambient cockpit/airframe noise remains
part of the excerpt. This is the requested A-10 sound style on the game's Gripen;
it does not represent a measured BK27 signature.

PCM is embedded into the executable and read by the bounded custom mixer.
No media downloads or decoding allocations occur during gameplay. Sound playback
loops continuously while actual rounds arrive and releases when firing stops;
the recorded GAU-8 cadence does not alter projectile count or game gun timing.
