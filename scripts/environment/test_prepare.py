#!/usr/bin/env python3
"""Offline tooling checks; no source downloads or game process required."""
import unittest, tempfile, importlib.util,sys
from pathlib import Path
import numpy as np
import rasterio
from rasterio.transform import from_origin
spec=importlib.util.spec_from_file_location('prepare',Path(__file__).with_name('prepare.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class Sampling(unittest.TestCase):
 def test_inclusive_grid_preserves_adjacent_sample_locations(self):
    with tempfile.TemporaryDirectory()as d:
        p=Path(d)/'source.tif'
        a=np.arange(121,dtype='float32').reshape(11,11)
        with rasterio.open(p,'w',driver='GTiff',height=11,width=11,count=1,dtype='float32',transform=from_origin(-.05,1.05,.1,.1),crs='EPSG:4326')as s:s.write(a,1)
        old=m.cached_source;m.cached_source=lambda u:p
        try:
            l=m.read_grid('local',(0,0,.5,1),11);r=m.read_grid('local',(.5,0,1,1),11)
            np.testing.assert_allclose(l[:,-1],r[:,0],atol=1e-4)
        finally:m.cached_source=old
if __name__=='__main__':unittest.main()
