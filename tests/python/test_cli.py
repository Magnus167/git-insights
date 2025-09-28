import io
import unittest
from contextlib import redirect_stdout
from importlib.metadata import version as pkg_version


def _load_ext():
    try:
        from git_insights import _git_insights
        return _git_insights
    except Exception:
        return None


class TestCLI(unittest.TestCase):
    def setUp(self) -> None:
        self.ext = _load_ext()
        if self.ext is None:
            self.skipTest("git_insights extension not available (run with pip install . or maturin develop --features python)")

    def test_help_exits_zero(self):
        code = self.ext.run(["git-insights", "--help"])
        self.assertEqual(code, 0)

    def test_version_exits_zero(self):
        code = self.ext.run(["git-insights", "--version"])
        self.assertEqual(code, 0)

    def test_version_prints_correct_version_and_pip_channel(self):
        expected_ver = pkg_version("git-insights")
        buf = io.StringIO()
        with redirect_stdout(buf):
            code = self.ext.run(["git-insights", "--version"])
        out = buf.getvalue()
        self.assertEqual(code, 0)
        self.assertIn(expected_ver, out)
        self.assertIn(" (pip)", out)


if __name__ == "__main__":
    unittest.main()
