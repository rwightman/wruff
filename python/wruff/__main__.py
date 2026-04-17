from __future__ import annotations

import os
import sys

from wruff import find_wruff_bin


def _run() -> None:
    wruff = find_wruff_bin()

    if sys.platform == "win32":
        import subprocess

        # Avoid emitting a traceback on interrupt
        try:
            completed_process = subprocess.run([wruff, *sys.argv[1:]])
        except KeyboardInterrupt:
            sys.exit(2)

        sys.exit(completed_process.returncode)
    else:
        os.execvp(wruff, [wruff, *sys.argv[1:]])


if __name__ == "__main__":
    _run()
