# Contributing Guide

`clap_schema` is developed and maintained by Selemis B.V.

Contributions from the community are welcome.

## Before contributing

For bug fixes, documentation improvements, tests, and other small, well-scoped changes, feel free to open a pull request directly.

For larger changes, new features or public API change, please open an issue first so the design can be discussed before substantial implementation work begins.

## Issues

If you encounter a bug, please check whether an existing issue already covers it before opening a new one.

Good bug reports include:

* a minimal reproduction where practical;
* the `clap_schema` and Rust versions involved;
* the expected and observed behavior;
* relevant logs or error messages;
* any investigation or root-cause analysis you have already done.

Feature proposals should explain the problem being solved, the intended behavior, and why it belongs in `clap_schema` rather than in application code or a separate integration.

## Development

Run the test suite with:

```sh
make test
```

Run linting and formatting checks with:

```sh
make lint
```

Build the documentation locally with:

```sh
make doc
```

The documentation build uses nightly rustdoc with all features enabled, the `docsrs` configuration, private items included, and warnings denied. It is intentionally stricter than the public documentation build on docs.rs.

Before submitting a pull request, run the complete repository verification:

```sh
make pr
```

This runs the repository's formatting, linting, tests, examples, doctests, documentation checks, and other verification steps.

## Pull requests

Keep pull requests focused on a single logical change.

Please include tests for behavioral changes and update documentation when public behavior or APIs change. New functionality should use `clap_schema`'s existing abstractions where possible rather than introducing parallel implementations.

Before submitting a pull request, run `make pr`. See [Development](#development) for the local development and verification workflow.

Pull requests may be asked to change substantially or be declined if the proposed design does not fit `clap_schema`'s scope, even when the implementation itself is correct.

## Compatibility

`clap_schema` is currently pre-1.0. Breaking changes may still be made when they materially improve the API.

## Security

If you believe you have found a security vulnerability, please do not report it through GitHub Issues or a public pull request.

See our [Security Policy](SECURITY.md) for reporting instructions.

## Licensing of contributions

`clap_schema` is dual licensed under the [Apache License, Version 2.0](LICENSE-APACHE) and [MIT license](LICENSE-MIT).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `clap_schema`, as defined in the Apache License 2.0, is licensed under
those same terms, without any additional terms or conditions.

By submitting a contribution, you represent that you have the right to submit
the contributed material under those terms.

If your contribution incorporates or is derived from third-party source
material, make that provenance clear in the pull request and preserve any
applicable license, copyright, and attribution requirements.
