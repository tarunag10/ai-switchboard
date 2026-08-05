# LLMLingua-2 promotion requirements

LLMLingua-2 remains `activationMode: experimental` and blocked for master activation until all gates below pass.

## Install and runtime

- Local model weights installed outside the repository.
- Documented memory ceiling for the target Mac (recommend ≥ 16 GB unified memory for comfortable local inference).
- Cold-start latency baseline recorded in `benchmarks/fixtures.json` or a linked local evidence summary.

## Quality baseline

- Wrong-omission rate must remain 0% on the golden benchmark fixtures with the engine enabled.
- Protected-content fixtures must stay byte-identical for tool JSON, file paths, and secret-like markers.
- Fail-open path must return the original prompt when compression exceeds the configured timeout.

## Measurement

- Shadow-mode token estimates only until a complete provider-billed counterfactual pair exists.
- Savings rows must stay labelled `estimated` until P1.4 counterfactual evidence is recorded for the same request class.

## Activation policy

- Master activation and max-compression flows must continue to exclude `llmlingua-2`.
- Promotion requires `npm run check:comprehensive-compression-plan` and `npm run check:world-class-benchmarks` to pass with the engine enabled in shadow fixtures only.
