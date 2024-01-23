**Zetch** is privately released to AWS CodeArtifact.

You should have been given an `<REGISTRY_KEY>` and `<REGISTRY_PASS>` to use. These can be used to produce an auth key.

Get an auth token and save it to the environment:

```bash
export ZETCH_PASS=`AWS_ACCESS_KEY_ID=<REGISTRY_KEY> AWS_SECRET_ACCESS_KEY=<REGISTRY_PASS> aws codeartifact get-authorization-token --domain zs-trading --domain-owner 428290171813 --region eu-central-1 --query authorizationToken --output text`
```

Auth tokens are only valid for 12 hours, so should be automated during install flows.
Hint: e.g. add this to your `.bashrc` / `.zshrc` so it always stays up to date.

Get the registry url:

```bash
# (treated later as <REGISTRY_URL>)
AWS_ACCESS_KEY_ID=<REGISTRY_KEY> AWS_SECRET_ACCESS_KEY=<REGISTRY_PASS> aws codeartifact get-repository-endpoint --domain zs-trading --domain-owner 428290171813 --repository zslib --format pypi --region eu-central-1 --query repositoryEndpoint --output text
```

The registry url is static, so can be hardcoded into your config files.

Installation method depends on your specific package manager. Common ones are listed below.

### Pdm

`pyproject.toml`

```toml
[tool.pdm.resolution]
respect-source-order = true

# Make sure the custom source is higher than the pypi one in the config,
# otherwise a public package with the same name will be chosen instead
[[tool.pdm.source]]
name = "zslib"
url = "aws:${ZETCH_PASS}@<REGISTRY_URL>"
verify_ssl = true

[[tool.pdm.source]]
name = "pypi"
url = "https://pypi.org/simple"
verify_ssl = true
```
