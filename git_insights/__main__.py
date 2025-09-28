import sys
from . import _git_insights

def main() -> None:
    code = _git_insights.run(list(sys.argv))
    raise SystemExit(code)

if __name__ == "__main__":
    main()
