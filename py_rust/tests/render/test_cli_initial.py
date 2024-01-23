import os

import pytest

from ..helpers.tmp_file_manager import TmpFileManager
from ..helpers.types import InputConfig
from ..helpers.utils import check_single


def test_cli_initial_fixes_circular_dep():
    """Make sure that using an initial breaks circular dependencies from cli vars."""

    def run(use_initial: bool):
        with TmpFileManager() as manager:
            circ_temp = manager.tmpfile("Hello, World!", full_name="circ_dep.zetch.txt")
            config: InputConfig = {
                "context": {
                    "cli": {
                        "VAR": {
                            "commands": [
                                "cat {}".format(os.path.join(manager.root_dir, "circ_dep.txt"))
                            ],
                        },
                    }
                }
            }

            if use_initial:
                config["context"]["cli"]["VAR"]["initial"] = "foo"

            check_single(
                manager,
                manager.create_cfg(config),
                "{{ VAR }}",
                "Hello, World!",
                extra_templates_written=[circ_temp],
            )

    # Should fail due to circular dependency without initial:
    with pytest.raises(ValueError, match="circ_dep.txt: No such file or directory"):
        run(False)

    # Should work with initial:
    run(True)
