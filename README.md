# codex-commit

[![check](https://trev.zip/llc/codex-commit/actions/workflows/check.yaml/badge.svg?branch=main&logo=forgejo&logoColor=%23bac2de&label=check&labelColor=%23313244)](https://trev.zip/llc/codex-commit/actions?workflow=check.yaml)
[![vulnerable](https://trev.zip/llc/codex-commit/actions/workflows/vulnerable.yaml/badge.svg?branch=main&logo=forgejo&logoColor=%23bac2de&label=vulnerable&labelColor=%23313244)](https://trev.zip/llc/codex-commit/actions?workflow=vulnerable.yaml)
[![rust](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Ftrev.zip%2Fllc%2Fcodex-commit%2Fraw%2Fbranch%2Fmain%2FCargo.toml&query=%24.package.rust-version&logo=rust&logoColor=%23bac2de&label=version&labelColor=%23313244&color=%23D34516)](https://releases.rs/)

generates commit messages with the codex cli

### example

```console
$ codex-commit
feat: add commit message generator
:
```

Press enter to use the generated message, or type a replacement at the `:`
prompt before committing.

## get

### nix

```sh
nix run git+https://trev.zip/llc/codex-commit.git
```

### download

https://trev.zip/llc/codex-commit/releases

---

> [!NOTE]
> This repository is mirrored to GitHub from [trev.zip](https://trev.zip/llc/codex-commit)
