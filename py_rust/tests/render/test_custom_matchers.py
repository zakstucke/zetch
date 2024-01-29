import re

import pytest

from ..helpers import cli
from ..helpers.tmp_file_manager import TmpFileManager
from ..helpers.types import InputConfig


@pytest.mark.parametrize(
    "config, files, templates",
    [
        ({}, ["foo.zetch"], ["foo.zetch"]),
        ({"matchers": ["zetch"]}, ["foo.zetch"], ["foo.zetch"]),
        (
            {"matchers": ["ree", "zetch", "bar"]},
            ["foo.zetch", "roo.ree", "other.other"],
            ["foo.zetch", "roo.ree"],
        ),
        # Matches both matchers but should still come out only once:
        ({"matchers": ["foo", "zetch"]}, ["foo.zetch"], ["foo.zetch"]),
        # Zetch shouldn't be hit when not specifically included:
        ({"matchers": ["zetchh", "ree"]}, ["foo.zetch", "foo.ree"], ["foo.ree"]),
        # lower case letters, numbers, dashes, underscores all should be allowed and work:
        *[
            (
                {"matchers": [matcher]},
                ["other.other", "foo.{}".format(matcher)],
                ["foo.{}".format(matcher)],
            )
            for matcher in ["kjkdf", "8787", "dfk_887-ksd"]
        ],
    ],
)
def test_custom_matchers(config: InputConfig, files: "list[str]", templates: "list[str]"):
    with TmpFileManager() as manager:
        for file in files:
            manager.tmpfile("", full_name=file)

        result = cli.render(manager.root_dir, config_file=manager.create_cfg(config))
        # Make sure the expected templates were found:
        assert sorted(result["debug"]["matched_templates"]) == sorted(templates)


def test_custom_matchers_invalid():
    # Error when "matchers" is an empty array, always need at least one matcher:
    with pytest.raises(
        ValueError,
        match=re.escape("[matchers]: must have at least one template matcher."),
    ):
        with TmpFileManager() as manager:
            cli.render(
                manager.root_dir,
                manager.tmpfile(
                    "matchers = []\n",
                    suffix=".toml",
                ),
            )

    # Error when "matchers" has not just lowercase letters, numbers, "-" & "_":
    for bad_matcher in [
        "foo.bar",
        ".ree",
        "/sds",
        "\\sds",
        "]]",
        "UPPER",
    ]:
        with pytest.raises(
            ValueError,
            match=re.escape("lowercase letters, numbers, dashes and underscores only in matchers"),
        ):
            with TmpFileManager() as manager:
                cli.render(
                    manager.root_dir,
                    manager.tmpfile(
                        "matchers = ['{}']\n".format(bad_matcher),
                        suffix=".toml",
                    ),
                )

        # Also confirm cannot be empty string:
        with pytest.raises(
            ValueError,
            match=re.escape("cannot be empty string"),
        ):
            with TmpFileManager() as manager:
                cli.render(
                    manager.root_dir,
                    manager.tmpfile(
                        "matchers = ['']\n",
                        suffix=".toml",
                    ),
                )
