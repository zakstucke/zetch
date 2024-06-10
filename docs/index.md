
# Zetch

[![License](https://img.shields.io/badge/License-MIT-green.svg)][license]
[![Documentation](https://img.shields.io/badge/Documentation-8A2BE2)](https://zakstucke.github.io/zetch)

[license]: https://github.com/zakstucke/zetch/blob/main/LICENSE.md

In-place, continuous templater.







[![PyPI](https://img.shields.io/pypi/v/zetch.svg)][pypi status]
[![Status](https://img.shields.io/pypi/status/zetch.svg)][pypi status]
[![Python Version](https://img.shields.io/pypi/pyversions/zetch)][pypi status]

[pypi status]: https://pypi.org/project/zetch/

You can install **Zetch** via [pip](https://pip.pypa.io/) from [PyPI](https://pypi.org/):

```console
pip install zetch
```

Binaries are available for:

* **Linux**: `x86_64`, `aarch64`, `i686`, `armv7`, `ppc64le`, `s390x`,  `musl-x86_64` & `musl-aarch64`
* **MacOS**: `x86_64`, `aarch64`
* **Windows**: `x86_64`, `aarch64`, `i686`

If your platform isn't supported, [file an issue](https://github.com/zakstucke/zetch/issues).

```
zetch: In-place, continuous templater.

Usage: zetch [OPTIONS] <COMMAND>

Commands:
  render           Render all templates found whilst traversing the given root (default)
  var              Read a finalised context variable from the config file
  read             Read sections of json/toml/yaml/yml files various file types from the command line, outputting in json
  put              Put/modify sections of json/toml/yaml/yml files, preserving comments and existing formatting where possible
  del              Delete sections of json/toml/yaml/yml files, preserving comments and existing formatting where possible
  init             Initialize the config file in the current directory
  replace-matcher  Replace a template matcher with another, e.g. zetch -> zet
  version          Display zetch's version
  help             Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  The config file to use. [default: ./zetch.config.toml]
  -h, --help             Print help
  -V, --version          Print version

Log levels:
  -v, --verbose  Enable verbose logging
  -s, --silent   Print diagnostics, but nothing else. Disable all logging (but still exit with status code "1" upon detecting diagnostics)

For help with a specific command, see: `zetch help <command>`.
```


## Usage

Please see the [documentation](https://zakstucke.github.io/zetch) for details.

## Contributing

Contributions are very welcome.
To learn more, see the [Contributor Guide](CONTRIBUTING.md).

## License

Distributed under the terms of the [MIT license](LICENSE.md),
**Zetch** is free and open source software.

## Issues

If you encounter any problems,
please [file an issue](https://github.com/zakstucke/zetch/issues) along with a detailed description.

