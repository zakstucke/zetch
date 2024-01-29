import os

import pytest

from ..helpers import cli
from ..helpers.tmp_file_manager import TmpFileManager


def test_cli_init():
    """Confirm the zetch init command works correctly.

    - Produces a valid file that can be used straight away.
    - Errs if default config already exists.
    - Includes .gitignore in ignore_files only if exists.
    """
    for use_gitignore in [True, False]:
        with TmpFileManager() as manager:
            if use_gitignore:
                manager.tmpfile("", full_name=".gitignore")

            cli.init(manager.root_dir)

            # Should have been created:
            config_path = os.path.join(manager.root_dir, "zetch.config.toml")
            assert os.path.exists(config_path)

            with open(config_path, "r") as file:
                contents = file.read()
                if use_gitignore:
                    assert 'ignore_files = [".gitignore"]' in contents
                else:
                    assert (
                        "ignore_files = [] # Couldn't find a .gitignore, not adding by default. Recommended if available."
                        in contents
                    )

            # The variables in the example conf loaded:
            manager.tmpfile("{{ FOO }} {{ BAR }} {{ BAZ }}", full_name="ree.zetch.txt")

            # Should run successfully:
            cli.render(manager.root_dir)

            # Should have been created:
            assert os.path.exists(os.path.join(manager.root_dir, "ree.txt"))
            with open(os.path.join(manager.root_dir, "ree.txt"), "r") as file:
                assert file.read() == "foo bar 1"

            # Should err second time as already exists:
            with pytest.raises(
                ValueError,
                match="Config file already exists at the default location: './zetch.config.toml'.",
            ):
                cli.init(manager.root_dir)


def test_schema_directive():
    """Confirm its at the top of the config file created with init, and updates itself on render when the directive changes."""
    with TmpFileManager() as manager:
        cli.init(manager.root_dir)
        prefix = "#:schema "

        config_path = os.path.join(manager.root_dir, "zetch.config.toml")
        with open(config_path, "r") as f:
            correct_contents = f.read()

        def get_directive() -> str:
            # Should have been created:
            with open(config_path, "r") as f:
                contents = f.read()

            first_line = contents.split("\n")[0]
            assert first_line.startswith(prefix), first_line
            return first_line[len(prefix) :]

        def assert_unchanged_config():
            with open(config_path, "r") as f:
                contents = f.read()
            assert contents.strip() == correct_contents.strip()

        def get_change_time() -> int:
            return os.stat(config_path).st_ctime_ns

        first_directive = get_directive()
        change_time = get_change_time()

        # Should run successfully, not update the config file as directive hasn't changed:
        cli.render(manager.root_dir)
        assert get_directive() == first_directive
        assert get_change_time() == change_time
        assert_unchanged_config()

        def fake_directive():
            with open(config_path, "w") as f:
                lines = correct_contents.split("\n")
                lines[0] = "{}foobar".format(prefix)
                f.write("\n".join(lines))

        # Should auto replace invalid:
        with open(config_path, "w") as f:
            lines = correct_contents.split("\n")
            lines[0] = "{}foobar".format(prefix)
            f.write("\n".join(lines))

        assert get_directive() == "foobar"
        cli.render(manager.root_dir)
        assert get_directive() == first_directive
        new_change_time = get_change_time()
        assert new_change_time > change_time
        assert_unchanged_config()

        # Shouldn't touch again:
        cli.render(manager.root_dir)
        assert get_change_time() == new_change_time
        assert_unchanged_config()
