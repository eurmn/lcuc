# Pengu Loader Research

## Finding

Pengu Loader is a community JavaScript plugin loader for the League of Legends client. Its public documentation describes plugin loading, runtime helpers, RCP hooks, and access to client-side functionality. The documentation and repositories reviewed do not establish Riot Games approval. Therefore, LCU Companion must not describe Pengu Loader as a Riot-approved SDK or make it a required part of the product.

References:

- [Pengu Loader welcome guide](https://pengu.lol/guide/welcome)
- [Pengu Loader runtime API](https://pengu.lol/runtime-api/)
- [Pengu Loader documentation repository](https://github.com/PenguLoader/docs)
- [Pengu Loader organization](https://github.com/PenguLoader)

## Relevant capabilities

- JavaScript plugin entry points and lifecycle hooks.
- Runtime helpers for plugin/client development.
- RCP/plugin hooks such as pre-init, post-init, and readiness callbacks.
- A client-embedded extension model that is separate from a desktop Tauri process.

These capabilities could support an optional in-client companion surface, but they do not replace the LCU REST/websocket collector and should not be used as the only source of match data.

## Recommended integration boundary

If evaluated later, package the Pengu Loader integration separately from the core application. It should exchange only an explicit, versioned local protocol with the LCU Companion Agent. The agent remains authoritative for collection and persistence. The plugin must be opt-in, disabled by default, and unable to access or forward LCU credentials.

Potential uses:

- Show lightweight rune recommendations inside the League client.
- Surface a link or notification when new statistics are available.
- Send user-initiated context such as the current champion or gameflow state to the agent.

Avoid:

- Making data collection depend on injected client code.
- Treating internal RCP APIs as stable public APIs.
- Shipping a plugin without clear user consent and compatibility checks.
- Logging or transmitting credentials, PUUIDs, or raw private responses.

## Decision

Pengu Loader is a future optional integration, not a dependency of the backend separation. The standalone agent will use the documented LCU lockfile, REST API, and websocket API first.

