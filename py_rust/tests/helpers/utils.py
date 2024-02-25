import os
import pathlib
import re
import typing as tp

from . import cli
from .tmp_file_manager import TmpFileManager

_LOCK_FILENAME = ".zetch.lock"
_MIDDLE_MATCHER = re.compile(r"\.zetch\.")
_END_MATCHER = re.compile(r"\.zetch$")


def get_out_path(path: pathlib.Path) -> tp.Optional[pathlib.Path]:
    middle_match = _MIDDLE_MATCHER.search(path.name)
    if middle_match is not None:
        return path.with_name(path.name.replace(middle_match.group(0), "."))

    end_match = _END_MATCHER.search(path.name)
    if end_match is not None:
        return path.with_name(path.name.replace(end_match.group(0), ""))

    return None


def get_lockfile_path(root: tp.Union[str, pathlib.Path]) -> pathlib.Path:
    return pathlib.Path(root).joinpath(f"./{_LOCK_FILENAME}")


def check_single(
    manager: TmpFileManager,
    config_file: tp.Union[str, pathlib.Path],
    contents: str,
    expected: tp.Union[str, tp.Callable[[str], bool]],
    file_type="txt",
    extra_args: tp.Optional["list[str]"] = None,
    ignore_extra_written: bool = False,
):
    template = manager.tmpfile(content=contents, suffix=".zetch.{}".format(file_type))

    rendered_info = cli.render(manager.root_dir, config_file, extra_args=extra_args)
    result = rendered_info["debug"]

    # Should return the correct compiled file:
    if not ignore_extra_written:
        expected_written = [remove_template(template)]
        assert result["written"] == expected_written, (
            result["written"],
            expected_written,
        )
        assert result["identical"] == [], result["identical"]

    # Original shouldn't have changed:
    with open(template, "r") as file:
        assert contents == file.read()

    # Compiled should match expected:
    out_path = get_out_path(template)
    assert out_path is not None
    with open(out_path, "r") as file:
        output = file.read()
        if isinstance(expected, str):
            assert output == expected, (output, expected)
        else:
            assert expected(output), (output, expected)


def remove_template(filepath: pathlib.Path) -> str:
    out_path = get_out_path(filepath)
    if out_path is None:
        raise ValueError(f"Could not find matcher in {filepath}")

    return str(out_path)


def str_path_for_tmpl_writing(path: pathlib.Path) -> str:
    r"""On windows paths in python have \\ rather than \'.

    But this will cause problems if writing to template in tests.
    This fn should wrap any paths being written to templates to handle edge cases.
    """
    if os.name == "nt":
        return str(os.path.normpath(path).replace("\\", "\\\\"))
    else:
        return str(os.path.normpath(path))


def cat_cmd_cross() -> str:
    if os.name == "nt":
        return "cmd.exe /c type"
    else:
        return "cat"


def no_file_err_cross(filename: str) -> str:
    if os.name == "nt":
        return "The system cannot find the file specified."
    else:
        return "{}: No such file or directory".format(filename)


def file_mod_time(filepath: str) -> tp.Optional[int]:
    """Return the last modified time of a file.

    If windows returns None as not supported, so should disable the test in usage.
    """
    if os.name == "nt":
        return None
    return os.stat(filepath).st_ctime_ns


def assert_file_modified_since(filepath: str, last: tp.Optional[int]):
    if last is not None:
        now = file_mod_time(filepath)
        assert now is not None and now > last, (now, last)


def assert_file_not_modified(filepath: str, last: tp.Optional[int]):
    if last is not None:
        now = file_mod_time(filepath)
        assert now is not None and now == last, (now, last)
