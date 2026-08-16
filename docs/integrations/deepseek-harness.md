# DeepSeek Harness developer preview

Status: **Experimental / Developer Preview**. AI Switchboard supports only the
exact upstream `dsh` CLI/schema snapshot below:

- package version: `0.1.0-rc.5`;
- upstream commit: `47f943859bef60e4160492346772ded9b24f765a`;
- repository: <https://github.com/deepseek-ai/deepseek-harness>;
- home-patch implementation: <https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/apps/cli/src/profile-boot.ts>;
- native DeepSeek LLM adapter schema: <https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/packages/llm/llm-deepseek/README.md>.

The adapter runs `dsh --version`, inspects only `$DSH_HOME/cordis.patch.yml`,
and never reads `$DSH_HOME/.credentials.yaml`. On the exact supported version
and a valid top-level Cordis PatchOptions sequence, an explicitly confirmed
dry-run may set only the native `llm-deepseek` plugin's documented `baseURL`
to `http://127.0.0.1:6767/v1`. This uses the supported home patch layer; it
does not patch DeepSeek Harness core. dsh keeps its existing
`DEEPSEEK_API_KEY` credential reference, model catalog, and user-visible model
selection.

Every changed pre-existing patch receives a sibling backup. Rollback restores
that exact backup. Off cleanup removes only the AI Switchboard marker block.
Unknown versions, malformed patches, and an existing user-owned
`ai-switchboard` route are guided-only and cannot write.
