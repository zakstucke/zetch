import typing as tp
from pathlib import Path

import pytest

from .helpers import cli
from .helpers.tmp_file_manager import TmpFileManager


@pytest.mark.parametrize(
    "desc, old_matcher, new_matcher, filename, changes_to",
    [
        ("simple_middle", "zetch", "etch", "foo.zetch.bar", "foo.etch.bar"),
        ("other_middle", "ree", "roo", "foo.ree.bar", "foo.roo.bar"),
        ("simple_end", "zetch", "etch", "foo.zetch", "foo.etch"),
        ("other_end", "ree", "roo", "foo.ree", "foo.roo"),
        ("partial_match_should_do_nothing_middle", "etch", "zetch", "foo.zetch.bar", None),
        ("partial_match_should_do_nothing_end", "etch", "zetch", "foo.bar.zetch", None),
        ("arb_file_should_ignore", "etch", "zetch", "arb.txt", None),
    ],
)
def test_replace_matcher(
    desc: str, old_matcher: str, new_matcher: str, filename: str, changes_to: tp.Optional[str]
):
    with TmpFileManager() as manager:
        contents = f"this is the content {old_matcher} {new_matcher}"
        original_file = manager.tmpfile(contents, full_name=filename)

        def run(stdin: tp.Optional[str]):
            cli.run(
                [
                    "zetch",
                    "replace-matcher",
                    old_matcher,
                    new_matcher,
                    "--config",
                    str(manager.create_cfg({})),
                ],
                custom_root=Path(manager.root_dir),
                stdin=stdin,
            )

        # Should abort if don't enter "y":
        run(None)
        run("n")
        # File should still exist and have the same contents:
        assert original_file.exists()
        assert original_file.read_text() == contents

        # Now it should work:
        run("y")

        if changes_to is None:
            # File should still exist and have the same contents:
            assert original_file.exists()
            assert original_file.read_text() == contents
        else:
            # File should have been replaced with changes_to and have the same contents:
            assert not original_file.exists()
            new_file = Path(manager.root_dir).joinpath(changes_to)
            assert new_file.exists()
            assert new_file.read_text() == contents
