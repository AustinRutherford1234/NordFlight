#!/usr/bin/env python3
"""Build offline Nord terrain packs using rasterio/GDAL. No credentials required.

pip install -r scripts/environment/requirements.txt
python scripts/environment/prepare.py --pack sweden-west
Source GeoTIFFs are accessed with HTTP ranges, not retained in the game assets.
The output is little-endian int16 height + u8 WorldCover on inclusive degree grids.
"""
import argparse
import hashlib
import json
from pathlib import Path
import time
import tempfile
import urllib.request
import urllib.error

import numpy as np
import rasterio
from rasterio.enums import Resampling
from rasterio.windows import from_bounds
from rasterio.warp import reproject
from rasterio.transform import from_origin

ROOT = Path(__file__).resolve().parents[2]
REGIONS = {"sweden-west": (10, 57, 15, 60), "norway-mountains": (6, 60, 10, 63)}


def cached_source(url):
    cache = Path(tempfile.gettempdir()) / "nord-terrain-sources"
    cache.mkdir(exist_ok=True)
    path = cache / Path(url).name
    if not path.exists():
        tmp = path.with_suffix(".download")
        urllib.request.urlretrieve(url, tmp)
        tmp.replace(path)
    return path


def read_grid(url, bounds, samples, categorical=False):
    # Inclusive sample locations: pad the read rectangle by half a sample so
    # adjoining tiles evaluate the exact same geographic border coordinates.
    w, s, e, n = bounds
    dx, dy = (e - w) / (samples - 1), (n - s) / (samples - 1)
    with rasterio.open(cached_source(url)) as src:
        result=np.zeros((samples,samples),dtype=src.dtypes[0])
        reproject(rasterio.band(src,1),result,src_transform=src.transform,src_crs=src.crs,
                  dst_transform=from_origin(w-dx/2,n+dy/2,dx,dy),dst_crs='EPSG:4326',
                  resampling=Resampling.nearest if categorical else Resampling.bilinear,
                  dst_nodata=0,num_threads=1)
        return result



def tile_url(lon, lat, resolution=30):
    name = f"Copernicus_DSM_COG_{10 if resolution == 30 else 30}_N{lat:02d}_00_E{lon:03d}_00_DEM"
    return f"https://copernicus-dem-{resolution}m.s3.amazonaws.com/{name}/{name}.tif"


def build_pack(name, samples, output, cover_samples=None):
    cover_samples = cover_samples or samples
    bounds = REGIONS[name]
    output.mkdir(parents=True, exist_ok=True)
    tiles = []
    for lat in range(bounds[1], bounds[3]):
        for lon in range(bounds[0], bounds[2]):
            stem = f"N{lat:02d}E{lon:03d}"
            path = output / f"{stem}.bin"
            source = tile_url(lon, lat)
            cover = ("https://esa-worldcover.s3.eu-central-1.amazonaws.com/v200/2021/map/"
                     f"ESA_WorldCover_10m_2021_v200_N{lat//3*3:02d}E{lon//3*3:03d}_Map.tif")
            if not path.exists():
                print(name, stem, "reading source grids", flush=True)
                try:
                    height = read_grid(source, (lon, lat, lon + 1, lat + 1), samples)
                except (rasterio.errors.RasterioIOError, urllib.error.HTTPError):
                    source = tile_url(lon, lat, 90)
                    height = read_grid(source, (lon, lat, lon + 1, lat + 1), samples)
                classes = read_grid(cover, (lon, lat, lon + 1, lat + 1), cover_samples, True).astype("u1")
                # WorldCover water has the DSM's flattened water elevation.
                # Suppress DSM canopy/building noise only by the chosen sample
                # resolution; this is explicitly NOT a surveyed bare-earth DTM.
                height = np.rint(np.nan_to_num(height)).clip(-32000, 32000).astype("<i2")
                payload = height.tobytes() + classes.tobytes()
                temporary = path.with_suffix(".tmp")
                temporary.write_bytes(payload)
                temporary.replace(path)
            payload = path.read_bytes()
            tiles.append(dict(file=path.name, west=lon, south=lat, east=lon + 1, north=lat + 1,
                              samples=samples, cover_samples=cover_samples, sha256=hashlib.sha256(payload).hexdigest(),
                              bytes=len(payload), elevation_source=source, landcover_source=cover))
    # Reconcile duplicate inclusive borders after independent source reads.
    # Both tiles must use identical samples, including classification.
    for t in tiles:
        n=t['samples'];nc=t['cover_samples'];a_path=output/t['file'];a=bytearray(a_path.read_bytes())
        heights=np.frombuffer(a,dtype='<i2',count=n*n).reshape(n,n)
        classes=np.frombuffer(a,dtype='u1',offset=n*n*2).reshape(nc,nc)
        for neighbor,axis in [((t['west']+1,t['south']),0),((t['west'],t['south']+1),1)]:
            other=next((q for q in tiles if (q['west'],q['south'])==neighbor),None)
            if other is None:continue
            b_path=output/other['file'];b=bytearray(b_path.read_bytes())
            bh=np.frombuffer(b,dtype='<i2',count=n*n).reshape(n,n)
            bc=np.frombuffer(b,dtype='u1',offset=n*n*2).reshape(nc,nc)
            if axis==0:
                # Use the nonzero source where source-border padding occurred.
                edge=np.where(heights[:,-1]==0,bh[:,0],heights[:,-1]);heights[:,-1]=edge;bh[:,0]=edge
                edge=np.where(classes[:,-1]==0,bc[:,0],classes[:,-1]);classes[:,-1]=edge;bc[:,0]=edge
            else:
                edge=np.where(heights[0,:]==0,bh[-1,:],heights[0,:]);heights[0,:]=edge;bh[-1,:]=edge
                edge=np.where(classes[0,:]==0,bc[-1,:],classes[0,:]);classes[0,:]=edge;bc[-1,:]=edge
            b_path.write_bytes(b)
        a_path.write_bytes(a)
    for t in tiles:t['sha256']=hashlib.sha256((output/t['file']).read_bytes()).hexdigest()
    manifest = dict(version=1, id=name, revision=2 if cover_samples!=samples else 1, bounds=bounds,
                    vertical_reference="EGM2008 metres interpreted as spherical mean-sea-level height",
                    ground_model="resampled DSM; vegetation/buildings are not fully removed",
                    tiles=tiles)
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(name, "complete", sum(t["bytes"] for t in tiles), "bytes", flush=True)


def global_base(output):
    # NOAA's 60 arcsecond global relief, reduced to 0.1 degree inclusive grid.
    url = ("https://www.ngdc.noaa.gov/mgg/global/relief/ETOPO2022/data/60s/60s_surface_elev_gtif/"
           "ETOPO_2022_v1_60s_N90W180_surface.tif")
    output.mkdir(parents=True, exist_ok=True)
    with rasterio.open(cached_source(url)) as src:
        a = src.read(1, out_shape=(1800, 3600), resampling=Resampling.average)
    a = np.rint(a).clip(-32000, 32000).astype("<i2")
    (output / "global.bin").write_bytes(a.tobytes())
    (output / "global.json").write_text(json.dumps(dict(version=1, width=3600, height=1800,
        source=url, vertical_reference="mean sea level", bytes=a.nbytes), indent=2) + "\n")


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--pack", choices=[*REGIONS, "global"], required=True)
    p.add_argument("--samples", type=int, default=1201,
                   help="Inclusive samples/degree: 1201 ~90m latitude; 3601 ~30m")
    p.add_argument("--cover-samples",type=int,default=2401,help="Categorical cover grid, independent of elevation resolution")
    p.add_argument("--output", type=Path)
    args = p.parse_args()
    if not 33 <= args.samples <= 3601:
        p.error("samples must be 33..3601")
    if not 33 <= args.cover_samples <= 7201:p.error("cover-samples must be 33..7201")
    output = args.output or ROOT / "assets/environment/packs" / args.pack
    with rasterio.Env(GDAL_CACHEMAX=64, GDAL_HTTP_TIMEOUT="60", GDAL_HTTP_MAX_RETRY="3"):
        if args.pack == "global":
            global_base(output)
        else:
            build_pack(args.pack, args.samples, output, args.cover_samples)
