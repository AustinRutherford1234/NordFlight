import json
from pathlib import Path
import tempfile
import unittest
from run import artifact_stamp,copy_fresh,identity,same_process
from verify import physical
from prune_artifacts import plan

class Validation(unittest.TestCase):
    def test_retention_preserves_baseline_accepted_and_unregistered_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory).resolve(); accepted=root/"accepted-final"
            for folder in (root/"baseline", accepted, root/"old-run"):
                folder.mkdir();(folder/"executable.json").write_text("{}")
                (folder/"visual-cockpit.png").write_bytes(b"image")
            (root/"old-run/my-photo.png").write_bytes(b"personal")
            (root/"visual-unregistered.png").write_bytes(b"personal")
            selected=plan(root,accepted)
            self.assertEqual(selected,[root/"old-run/visual-cockpit.png"])
            with self.assertRaises(ValueError):plan(root,root.parent/"accepted-other")
    def test_retention_does_not_select_children_of_retired_trace(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory).resolve();accepted=root/"accepted-final";accepted.mkdir()
            profile=root/"old-profile";profile.mkdir();(profile/"capture.json").write_text("{}")
            trace=profile/"profile.trace";trace.mkdir();(trace/"frame-000.png").write_bytes(b"capture")
            self.assertEqual(plan(root,accepted),[trace])
    def test_stale_capture_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            p=Path(directory)/"capture.png";p.write_bytes(b"stale")
            with self.assertRaises(RuntimeError):copy_fresh(p,Path(directory),artifact_stamp(p),0)
    def test_comparison_excludes_wall_time_but_preserves_simulation_time_and_rng(self):
        a={"wall_seconds":1.,"seconds":100.,"rng":147}
        b={"wall_seconds":5.,"seconds":100.,"rng":147}
        self.assertEqual(physical("bvr-study.json",a),physical("bvr-study.json",b))
        b["seconds"]=99.
        self.assertNotEqual(physical("bvr-study.json",a),physical("bvr-study.json",b))
        b["seconds"]=100.;b["rng"]=148
        self.assertNotEqual(physical("bvr-study.json",a),physical("bvr-study.json",b))
    def test_recovery_verifies_process_identity(self):
        import os
        record=identity(os.getpid());self.assertTrue(same_process(record))
        record["identity"]+="wrong executable";self.assertFalse(same_process(record))

if __name__=="__main__":unittest.main()
