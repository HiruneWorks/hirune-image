# AVIF native adapter

This directory vendors the public bindings and build helpers from `jc-libavif-sys` 1.3.0-rc2.
Upstream: <https://github.com/jcayzac/jc-libavif-sys>.
The bindings use the upstream MIT OR Apache-2.0 license. Keep both license files.

The project builds these fixed decoder sources:

| Source | Version | SHA-256 |
| --- | --- | --- |
| libavif | 1.3.0 | `0a545e953cc049bf5bcf4ee467306a2f113a75110edf59e61248873101cd26c1` |
| libaom | 3.13.1 | `19e45a5a7192d690565229983dad900e76b513a02306c12053fb9a262cbeca7d` |

The local changes:

- Set source-only builds in `.cargo/config.toml`.
- Check each source archive hash before extraction. Cache archives under `target/native-sources`.
- Use the official `aom-releases.storage.googleapis.com` bucket endpoint.
- Build decoder-only libaom. Disable the AVIF encoder.
- Read full NASM help for NASM 2/3 compatibility.
- Stage MSVC's `Release/aom.lib` where libavif expects its local codec.
- Link static libaom explicitly on the source build path.
- Check build cleanup paths against `OUT_DIR`. Copy source trees without directory rename races on Windows.

The first build downloads source archives. The running service performs no codec downloads.
The runtime image contains the native notices in `licenses/`.
When updating a codec, update its version, archive hash, generated bindings, notices, and functional tests together.
