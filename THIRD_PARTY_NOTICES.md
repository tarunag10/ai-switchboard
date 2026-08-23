# AI Switchboard third-party notices

AI Switchboard is developed for private research and is published as open
source. No commercial or monetary use is intended by the project. This intent
does not remove or replace any upstream licence, copyright, attribution, NOTICE,
patent, dependency, model, or trademark requirement.

The authoritative machine-readable delivery and migration status is
[`third_party/oss-integrations.json`](third_party/oss-integrations.json).

## Code currently derived or embedded

- **Headroom** — MIT. Portions of the desktop shell and integration approach
  are derived from Headroom. Preserve the upstream copyright and MIT licence;
  see [`NOTICE`](NOTICE) and the [upstream project](https://github.com/headroomlabs-ai/headroom).
- **Caveman** — AI Switchboard-native guidance. No external runtime or upstream
  source is embedded.
- **Switchboard Pack Compaction** — AI Switchboard-native deterministic,
  no-model pack compaction. The persisted value `chonkify` is retained only as
  a backwards-compatible preference ID. No upstream Chonkify source is embedded.

## Upstream projects referenced or externally installed today

- **DeepSeek Harness** — MIT, developer preview; exact evaluated commit
  `47f943859bef60e4160492346772ded9b24f765a`.
- **NVIDIA NeMo Switchyard** — Apache-2.0. Any copied code must also reproduce
  the upstream `NOTICE` and identify modifications.
- **Ponytail** — MIT. The current mutable host-plugin installation is scheduled
  for replacement by reviewed app resources.
- **Microsoft MarkItDown** — MIT. Its optional dependency set has independent
  licences and is not yet approved for bundling as a whole.
- **leanctx** — MIT. Optional dependencies and model weights require separate
  review.
- **RTK** — Apache-2.0 according to the evaluated upstream licence. Exact
  source-to-release provenance and licence metadata must be resolved before
  source vendoring.
- **JCode** — reference only. Conflicting repository identities must be resolved
  before copying source or assigning attribution.
- **Upstream Chonkify** — reference only and not the implementation shipped by
  AI Switchboard. No upstream Chonkify code is copied into this repository.

Full upstream licence texts and copyright notices must be added to the app
bundle in the same phase that any corresponding source or binary is vendored.
The current repository `LICENSE`, `NOTICE`, this notice index, and the canonical
OSS inventory are bundled with the desktop application.
