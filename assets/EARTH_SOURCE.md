# Earth color imagery

Credit: Reto Stöckli / NASA Earth Observatory, Blue Marble: Next Generation,
July 2004 monthly cloud-free composite.

Source catalog: https://science.nasa.gov/earth/earth-observatory/blue-marble-next-generation/base-map/

- `earth.jpg`: global 8192 × 4096 JPEG, resized from NASA's
  `world.200407.3x21600x10800.jpg`, JPEG quality 94.
  https://assets.science.nasa.gov/content/dam/science/esd/eo/images/bmng/bmng-base/july/world.200407.3x21600x10800.jpg
- `earth-nordic.jpg`: 4096 × 3511 JPEG covering longitude 0°–35° E,
  latitude 45°–75° N. Cropped from NASA's 500 m C1 tile at source pixel
  offset (x=0, y=3600), width 8400 and height 7200, then resized to width
  4096 with macOS `sips`, JPEG quality 94.
  https://assets.science.nasa.gov/content/dam/science/esd/eo/images/bmng/bmng-base/july/world.200407.3x21600x21600.C1.jpg
- `earth-clouds.jpg`: NASA Blue Marble cloud composite, 2048 × 1024.
  NASA Goddard Space Flight Center / Reto Stöckli. Brightness becomes cloud
  opacity at load time; clear pixels remain transparent.
  https://eoimages.gsfc.nasa.gov/images/imagerecords/57000/57747/cloud_combined_2048.jpg
  Original catalog: https://visibleearth.nasa.gov/images/57747/blue-marble-clouds/77558l

The application lifts very dark water candidates for ocean/lake readability, and shades
both layers with the same day/night function. The Nordic layer fades to
transparent along its border. These are color images; terrain elevation,
landable detail and atmospheric limb effects remain game approximations.
The cloud composite is static historical imagery, not current weather or a
volumetric weather simulation. Existing near-ground cumulus remains procedural.
