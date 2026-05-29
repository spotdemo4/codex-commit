# rust template

[![check](https://trev.zip/llc/rust/actions/workflows/check.yaml/badge.svg?branch=main&logo=forgejo&logoColor=%23bac2de&label=check&labelColor=%23313244)](https://trev.zip/llc/rust/actions?workflow=check.yaml)
[![vulnerable](https://trev.zip/llc/rust/actions/workflows/vulnerable.yaml/badge.svg?branch=main&logo=forgejo&logoColor=%23bac2de&label=vulnerable&labelColor=%23313244)](https://trev.zip/llc/rust/actions?workflow=vulnerable.yaml)
[![rust](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Ftrev.zip%2Fllc%2Frust%2Fraw%2Fbranch%2Fmain%2FCargo.toml&query=%24.package.rust-version&logo=rust&logoColor=%23bac2de&label=version&labelColor=%23313244&color=%23D34516)](https://releases.rs/)

template for starting [rust](https://rust-lang.org/) projects

part of [spotdemo4/templates](https://github.com/spotdemo4/templates)

## requirements

- [nix](https://nixos.org/)

## getting started

```sh
nix develop
```

### run

```sh
nix run #dev
```

### format

```sh
nix fmt
```

### check

```sh
nix flake check
```

### build

```sh
nix build
```

### release

```sh
bumper "README.md"
```

releases are created automatically for [significant](https://www.conventionalcommits.org/en/v1.0.0/#summary) changes

## use

### docker

```sh
docker run ghcr.io/spotdemo4/rust-template:latest
```

### nix

```sh
nix run github:spotdemo4/rust-template
```

### download

https://trev.zip/llc/rust/releases

---

> [!NOTE]
> This repository is mirrored to GitHub from [trev.zip](https://trev.zip/llc/rust)
