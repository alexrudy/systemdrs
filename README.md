# SystemD-rs

A quick library for interacting with SystemD from inside a rust library.

This is meant to be a re-implementation of `libsystemd` in rust, with a more idiomatic API, but it is definitely imperfect and missing most features.

Right now, it covers:
- `sd_notify` (notify systemd of readiness)
- `sd_listen_fds` (get the number of file descriptors passed to the process)
