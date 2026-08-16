# Exact Response Cache contract and migration boundary

AI Switchboard's shipping cache is the **Exact Response Cache** (short label:
**Response Cache**). It replays a locally stored response only for a safely
namespaced request identity. It is not a semantic cache and makes no embedding,
vector-distance, or meaning-based match.

The canonical backend diagnostics command is
`get_response_cache_diagnostics`. It returns only operational facts: enabled
state, matching strategy, aggregate entries/hits/misses, opaque SHA-256
namespace identifiers, storage path, bypass and safety rule identifiers, and
the clear/restore action contract. It never returns prompts, responses, cache
keys, account names, or workspace paths.

Clearing through `clear_response_cache` requires the exact phrase
`clear exact response cache`. Clear deletes cached response bodies and retains
no rollback copy. Its receipt reports the deleted entry count and explicitly
sets `restoreAvailable` to false; the diagnostics clear action sets
`retainsLocalRestoreCopy` to false. Clearing does not toggle the user's enabled
setting, so the cache can safely refill after a clear or be disabled separately.

## Compatibility retained for one migration window

The Rust aliases `SemanticCache` and `SemanticCacheService`, legacy Tauri
commands beginning `*_semantic_cache_*`, the `semantic_cache` SQLite tables,
the `semantic-cache.sqlite3` and `semantic-cache.json` filenames, and the
serialized `semanticV2Enabled` field remain readable so existing installs and
frontends do not lose state. These are compatibility identifiers, not product
terminology.

The canonical add-on profile ID is `response-cache`; `semantic-cache` remains
an accepted backend alias during this migration window.

The historical v2 flag implemented a normalized lexical prefix key, not true
semantic similarity. On load it is migrated to false, attempts to enable it are
rejected, and canonical diagnostics always report exact SHA-256 request
identity matching. A future true semantic cache requires a separate experiment,
storage/safety contract, similarity threshold, and benchmark gate.
