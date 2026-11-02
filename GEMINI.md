# GEMINI.md — Zero-Error Engineering & Agent Workflow Guide

## 1. Mission & Engineering Philosophy

**LCU Companion** is a high-performance Tauri 2 desktop application (Rust backend + SQLite + Svelte 5 frontend) for League of Legends, specializing in real-time LCU synchronization and the **ARAM Mayhem Build & Synergy Engine**.

Every agent operating in this repository must prioritize **correctness, numerical stability, type safety across the IPC boundary, and 60 FPS UI responsiveness**. Never ship placeholder logic, unguarded floating-point math, or unverified cross-language DTO changes.

---

## 2. The 5-Stage Clean-Delivery Workflow

To finish features cleanly without regressions, broken builds, or race conditions, execute all tasks through this strict, sequential pipeline:

### Stage 1: Precision Recon (Read-Only Discovery)
Before proposing or writing any code, locate exact symbols, call graphs, and database schemas.
- **Do not read entire large files** into context.
- Use **LSP** (`workspaceSymbol`, `goToDefinition`, `findReferences`) and **AST search** (`ast-grep`) to inspect Rust structs, Tauri commands, and SQLite queries.
- Use **`type-inject`** (`search_symbols`, `get_file_outline`, `lookup_type`) to inspect TypeScript interfaces and Svelte 5 component props.
- Identify all upstream callers and downstream consumers of any symbol you plan to touch.

### Stage 2: Cross-Boundary Contract Lock (Rust $\leftrightarrow$ TypeScript)
The #1 source of integration bugs in Tauri apps is drift between Rust `serde` structs and TypeScript interfaces. Lock the contract **before** writing business logic:
1. Define or update the Rust DTO structs with explicit `#[derive(Debug, Clone, Serialize, Deserialize)]` and consistent `#[serde(rename_all = "camelCase")]`.
2. Update the corresponding TypeScript types and IPC wrapper signatures in `src/lib/api/`.
3. Verify that all optional/nullable fields (`Option<T>` $\leftrightarrow$ `T | null`) and numeric types (`i64`/`f64` $\leftrightarrow$ `number`) match identically.

### Stage 3: Backend & Statistical Engine Execution (Rust + SQLite)
Implement all domain logic, database aggregations, and scoring algorithms strictly in Rust (`src-tauri/src/`).
- **Concurrency & DB Safety**:
  - Maintain strict separation between `agent.db` (personal data) and `global-aram.db` (global crawler & Mayhem dataset).
  - Use short-lived, WAL-enabled connections with a 5-second busy timeout.
  - **Never** hold a SQLite transaction or Mutex lock across an `.await` point, LCU HTTP call, or heavy JSON deserialization.
  - Wrap synchronous `rusqlite` operations inside bounded `tokio::task::spawn_blocking` tasks.
- **Numerical & Statistical Stability**:
  - Guard every division against zero denominators ($n = 0$).
  - Clamp probabilities to $[\epsilon, 1 - \epsilon]$ where $\epsilon = 10^{-6}$ before computing log-odds:
    $$\text{logit}(p) = \ln\left(\frac{p}{1 - p}\right)$$
  - Filter out non-build items (e.g., Poro-Snax IDs `2050` and `2052`) at ingestion and query time.
  - Canonicalize unordered pairs and triplets by sorting entity IDs ($id_1 < id_2 < id_3$) so $(A, B)$ and $(B, A)$ never split sample counts.

### Stage 4: Reactive Frontend Implementation (Svelte 5 + TypeScript)
Keep the frontend thin. Svelte components must only manage reactive view state, user interaction, and presentation of pre-computed Rust DTOs.
- **Strict Svelte 5 Runes**: Use `$props()`, `$state()`, `$derived()`, `$derived.by()`, and `$effect()`. Never introduce legacy Svelte 3/4 syntax (`export let`, `$:`, or writable stores) in new or refactored code.
- **Flicker-Free Optimistic UI & Debounced IPC**:
  - When user interactions trigger backend recomputation (e.g., slotting an item or augment in the simulator), update local UI state immediately.
  - Debounce rapid IPC calls (150ms window) and guard responses with a monotonically increasing `requestId` so out-of-order responses never overwrite newer state.
  - Retain a `lastGoodRec` state snapshot during pending recalculations to prevent layout shift or empty flashes.
- **DOM Virtualization**: Any list or combo table capable of rendering $>50$ rows (such as 2-way/3-way synergy matrices) must use fixed-row-height window virtualization ($~15$ visible DOM nodes + overscan) to guarantee 60 FPS scrolling.

### Stage 5: Zero-Error Verification Gate
No task is complete until every modified file passes static verification:
1. **Rust Verification**: Query `rust-analyzer` LSP diagnostics on all modified `.rs` files. Resolve all compiler errors, borrow-checker issues, and unused import/variable warnings.
2. **TypeScript & Svelte 5 Verification**:
   - Run `svelte` MCP (`svelte-autofixer`) on every touched `.svelte` component to catch rune misuse or reactivity pitfalls.
   - Run `type-inject` (`type_check`) / TypeScript LSP diagnostics to confirm zero type mismatches across `src/lib/api/` and components.
3. **Edge-Case Audit**: Trace execution paths for cold-start states (empty database, 0 games recorded for a champion, missing CommunityDragon icon asset, disconnected LCU client). Ensure every fallback renders gracefully.

---

## 3. ARAM Mayhem Build Engine Invariants

When implementing or tuning the recommendation and synergy engine, adhere strictly to these mathematical specifications:

1. **Hierarchical Empirical Bayes Shrinkage**:
   Never sort or recommend entities by raw sample winrate. Shrink observed winrates toward the contextual baseline prior $p_0$ using pseudocount weight $M$:
   $$\hat{p}_{\text{EB}} = \frac{w + M \cdot p_0}{n + M}$$
   where $w$ is observed wins and $n$ is observed matches.

2. **True Synergy Interaction ($\Delta\beta_{ij}$) vs. Additive Winrate**:
   To detect genuine combos (Augment + Item, Augment + Augment, Item + Item, or 3-way Champion + Augment + Item), isolate the interaction lift above individual main effects in log-odds space:
   $$\text{logit}(P(\text{Win} \mid c, A, B)) = \beta_c + \Delta\beta_{c,A} + \Delta\beta_{c,B} + \Delta\beta_{A,B} + \Delta\beta_{c,A,B}$$
   A combo is only flagged as a true synergy if the interaction term $\Delta\beta_{A,B}$ (or $\Delta\beta_{c,A,B}$) is positive after uncertainty penalization.

3. **Lower Confidence Bound (LCB) Ranking**:
   Rank candidate builds, items, and combos by their conservative Lower Confidence Bound so noisy $2\text{-match}$ outliers never outrank proven $500\text{-match}$ staples:
   $$\text{Score}_{\text{LCB}} = \hat{\mu} - z \cdot \frac{\sigma}{\sqrt{n + M}}$$

4. **Context-Aware 3-Choice Augment Card Scorer**:
   At levels $1, 7, 11,$ and $15$, the player selects $1$ of $3$ augments of a fixed tier (Silver, Gold, or Prismatic). Score each candidate $A_k \in \{A_1, A_2, A_3\}$ conditioned on the active build state $\mathcal{S} = (c, \mathcal{A}_{\text{owned}}, \mathcal{I}_{\text{owned}})$:
   $$\text{EV}(A_k \mid \mathcal{S}) = \Delta\text{WPA}_{\text{base}}(c, A_k) + \sum_{a \in \mathcal{A}_{\text{owned}}} \text{Synergy}(c, A_k, a) + \sum_{i \in \mathcal{I}_{\text{owned}}} \text{Synergy}(c, A_k, i) + \text{Bonus}_{\text{Set}}(\mathcal{A}_{\text{owned}} \cup \{A_k\}) + \text{MaxPath}(A_k, \mathcal{I}_{\text{unbuilt}})$$
   where $\text{Bonus}_{\text{Set}}$ captures the non-linear breakpoint value of completing a $2/3/4$-piece Mayhem Augment Set, and $\text{MaxPath}$ scores the best unlocked future item purchase.

5. **Personal + Global Blend**:
   When combining `agent.db` (personal matches $n_p$) with `global-aram.db` (global matches $n_g$), compute a precision-weighted posterior where personal performance gradually overrides the global prior as $n_p$ grows:
   $$\theta_{\text{blend}} = \alpha(n_p) \cdot \hat{\theta}_{\text{personal}} + \left(1 - \alpha(n_p)\right) \cdot \hat{\theta}_{\text{global}}, \quad \alpha(n_p) = \frac{n_p}{n_p + \kappa}$$

---

## 4. Multi-Agent Coordination & Delegation Rules

When operating in a multi-agent setup (`orchestrator`, `architect`, `scout`, `worker`, `reviewer`), follow these rules to prevent context pollution and merge conflicts:

- **Strict Disjoint File Ownership**: Never assign the same file to two active `worker` agents simultaneously. Divide work cleanly along architectural seams (e.g., Worker A owns `src-tauri/src/agent/mayhem_engine.rs`, Worker B owns `src/lib/components/history/MayhemBuildSimulator.svelte`).
- **Surgical Subagent Prompts**: Pass only exact file paths, line ranges, symbol signatures, and the locked Stage 2 contract. Never paste raw multi-hundred-line files into subagent prompts.
- **Explicit Tool Directives**:
  - Rust code & crates $\rightarrow$ `lsp` (`rust-analyzer`), `ast-grep`, `rust-docs` MCP.
  - TypeScript types $\rightarrow$ `type-inject` (`lookup_type`, `list_types`, `type_check`).
  - Svelte 5 components $\rightarrow$ `svelte` MCP (`svelte-autofixer`, `get-documentation`).
  - UI styling & icons $\rightarrow$ `flowbite-svelte` primitives, `@iconify-json/heroicons` classes (`icon-[heroicons--...]`), and official Mayhem tier colors (Silver `#a3b9bd`, Gold `#e3c49f`, Prismatic `#f2c2ff` with border gradient `linear-gradient(180deg, #e78fff, #70008f)`).
- **Command & Package Discipline**:
  - Always use `bun` (never `npm` or `pnpm`) and Windows backslash paths (`\`) where OS-specific.
  - Use LSP diagnostics and MCP validators continuously during development; do not spawn heavy shell builds (`cargo test`, `cargo check`, `vite build`) unless explicitly requested by the user.

---

## 5. Definition of Done (Pre-Handoff Checklist)

Before marking any task complete, verify every item below:

- [ ] **Contract Parity**: Rust `serde` structs and TypeScript interfaces in `src/lib/api/` match field-for-field.
- [ ] **Zero Diagnostics**: `rust-analyzer`, `type-inject`, and `svelte-autofixer` report 0 errors on all modified files.
- [ ] **Math Safety**: All winrate/WPA/synergy calculations apply Empirical Bayes shrinkage, LCB uncertainty penalties, and zero-division guards.
- [ ] **Non-Blocking Execution**: All SQLite reads/writes execute inside `spawn_blocking` with short transactions, and all high-frequency UI interactions are debounced (150ms) with stale-request guards.
- [ ] **Visual & Asset Integrity**: Lowercase CommunityDragon asset paths are preserved, local disk cache URIs are respected, and missing icons degrade gracefully to fallback glyphs.