import pytest
import zetch

from ..helpers import cli


@pytest.mark.parametrize(
    "args",
    [
        # Make sure all supported versions act the same:
        ["--version"],
        ["-V"],
        ["version"],
    ],
)
def test_cli_version(args: list[str]):
    """Confirm the zetch version command works correctly."""
    res = cli.run(["zetch", *args])
    assert res.startswith("zetch {}".format(zetch.__version__)), res
    # Should be including the path to the executable in brackets at the end:
    assert res.endswith("zetch)"), res
