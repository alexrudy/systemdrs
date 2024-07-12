# SystemD-connector

[![crate][crate-image]][crate-link]
[![Docs][docs-image]][docs-link]
[![Build Status][build-image]][build-link]
![MIT licensed][license-image]

A quick library for interacting with SystemD from inside a rust library.

This is meant to be a re-implementation of `libsystemd` in rust, with a more idiomatic API, but it is definitely imperfect and missing most features.

Right now, it covers:
- `sd_notify` (notify systemd of readiness)
- `sd_listen_fds` (get the number of file descriptors passed to the process)

[crate-image]: https://buildstats.info/crate/systemd-connector
[crate-link]: https://crates.io/crates/systemd-connector
[docs-image]: https://docs.rs/systemd-connector/badge.svg
[docs-link]: https://docs.rs/systemd-connector/
[build-image]: https://github.com/alexrudy/systemd-connector/actions/workflows/ci.yml/badge.svg
[build-link]: https://github.com/alexrudy/systemd-connector/actions/workflows/ci.yml
[license-image]: https://img.shields.io/badge/license-MIT-blue.svg
