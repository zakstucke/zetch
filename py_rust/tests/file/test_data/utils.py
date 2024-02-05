import os


def tfile(filename: str) -> str:
    with open(os.path.join(".", "tests", "file", "test_data", filename)) as f:
        return f.read()
