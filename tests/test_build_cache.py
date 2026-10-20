"""Exercise packaging/cache policy without compiling or touching real artifacts."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


class BuildCacheTests(unittest.TestCase):
    def run_build(self, initial_kib, built_kib):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            script = Path(__file__).resolve().parents[1] / "build-mac.sh"
            shutil.copy2(script, root / script.name)
            (root / "target").mkdir()
            (root / "cache-size").write_text(str(initial_kib))
            shims = root / "bin"
            shims.mkdir()
            commands = {
                "du": '#!/bin/sh\ncat cache-size\n',
                "cargo": '''#!/bin/sh
echo "$*" >> calls
if [ "$1" = clean ]; then
  rm -rf target
  echo 0 > cache-size
else
  mkdir -p target/release
  echo executable > target/release/gripen-flight
  echo "$BUILT_KIB" > cache-size
fi
''',
                "strip": '#!/bin/sh\nexit 0\n',
                "codesign": '#!/bin/sh\necho signed >> calls\n',
            }
            for name, content in commands.items():
                path = shims / name
                path.write_text(content)
                path.chmod(0o755)
            environment = dict(os.environ, PATH=f"{shims}:/usr/bin:/bin",
                               BUILT_KIB=str(built_kib), ZDOTDIR=str(root))
            subprocess.run(["/bin/zsh", str(root / script.name), "release"],
                           env=environment, check=True, capture_output=True)
            calls = (root / "calls").read_text().splitlines()
            self.assertEqual((root / "Nord Flight.app/Contents/MacOS/gripen-flight")
                             .read_text(), "executable\n")
            return calls

    def test_small_cache_retained(self):
        self.assertEqual(self.run_build(1024, 2048),
                         ["build --profile release --locked", "signed"])

    def test_oversized_cache_cleared_before_build(self):
        self.assertEqual(self.run_build(13 * 1024 * 1024, 2048),
                         ["clean", "build --profile release --locked", "signed"])

    def test_oversized_build_cleared_only_after_packaging(self):
        self.assertEqual(self.run_build(1024, 13 * 1024 * 1024),
                         ["build --profile release --locked", "signed", "clean"])


if __name__ == "__main__":
    unittest.main()
