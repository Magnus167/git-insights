from importlib.metadata import version as pkg_version


def _load_ext():
    try:
        from git_insights import _git_insights
        return _git_insights
    except Exception:
        return None



def test_help_exits_zero():
    ext = _load_ext()
    if ext is None:
        return
    assert ext.run(["git-insights", "--help"]) == 0


def test_version_exits_zero():
    ext = _load_ext()
    if ext is None:
        return
    assert ext.run(["git-insights", "--version"]) == 0

def test_version_prints_correct_version_and_pip_channel(capsys):
    ext = _load_ext()
    if ext is None:
        return
    expected_ver = pkg_version("git-insights")
    code = ext.run(["git-insights", "--version"])
    captured = capsys.readouterr()
    assert code == 0
    assert expected_ver in captured.out
    assert " (pip)" in captured.out
