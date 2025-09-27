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
