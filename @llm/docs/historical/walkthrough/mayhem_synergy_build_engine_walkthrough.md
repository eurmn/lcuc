# Walkthrough: ARAM Mayhem Hierarchical Bayes Build Engine, Packed `u64` Combo Miner & Live 3-Card Augment Advisor

## 1. Project Context & Objectives

**Task ID**: `task-mayhem-synergy-build-engine`  
**Application**: LCU Companion (Tauri v2 Desktop App — Rust Backend + SQLite + Svelte 5 Frontend)

### Core Objectives:
1. **Packed `u64` Key Layout & Single-Pass N-Gram Miner**:
   - 64-bit bitmask encoding `(kind: 4-bit, champ_id: 16-bit, entity_a: 22-bit, entity_b: 22-bit)`.
   - Single-pass n-gram participant aggregation filtering out non-build items (Poro-Snax IDs `2050`, `2052`, `220013`).
   - Canonical entity ordering ($entity_a \le entity_b$) to prevent count splitting.
2. **4-Level Hierarchical Empirical Bayes & Wilson LCB ($z = 1.28155$)**:
   - Tier/slot survivorship normalization ($\delta_{\text{tier}}(r)$) for augment tiers (Silver, Gold, Prismatic) and items.
   - Champion baseline prior ($m_c = 30.0$).
   - Single-resource posterior ($m_1 = 25.0$) blended with personal match history ($m_{\text{pers}} = 10.0$).
   - Kit-specific synergy lift ($\delta_{\text{champ}}(r, c) = \text{WPA}(r \mid c) - \text{WPA}_{\text{glob}}(r)$).
   - 80% Wilson Lower Confidence Bound ranking with zero-division guards.
3. **Pairwise & 3-Way Multi-Resource Synergy Lift ($\Delta_{\text{syn}}$)**:
   - Additive independence baseline $p_{\text{indep}}(x, y \mid c)$.
   - Global combo lift with Kit-Eligibility Gating ($N_{xc} > 0 \land N_{yc} > 0$).
   - Pure synergy lift $\Delta_{\text{syn}} = p_{\text{combo}} - p_{\text{indep}}$ and credible LCB synergy lift.
4. **Log-Odds Damped Build Simulator & 3-Card Augment Recommender**:
   - Log-odds summation with active pairwise synergy decay ($\gamma = 0.70$) and smooth hyperbolic tangent saturation ($L_{\text{max}} = 1.50$).
   - Slot-aware lookahead weight ($\lambda_{\text{look}} \in [0.65, 0.45, 0.25, 0.10]$) with combo partner unlock detection.
   - Non-repeating reroll expected value ($\text{EV}_{\text{reroll}}$) and upgrade probability over unseen pool $U_{T, c}$.
5. **High-Performance Svelte 5 Frontend**:
   - `MayhemAugmentAdvisorCard.svelte`: 3-card evaluation cards, dynamic BEST PICK vs REROLL badges, upgrade probability, kit lift, active synergies, and lookahead unlock callouts.
   - `MayhemCombosCard.svelte`: Virtualized fixed-row-height matrix rendering ~15 visible DOM nodes with category pill filtering.
   - `MayhemBuildSimulator.svelte`: Optimistic slot simulation with pending indicators.
   - `MayhemChampSelectView.svelte`: 150ms IPC debouncing, `lastGoodRec` stale response protection, live client inventory auto-sync, and log-odds Build Power Bar.

---

## 2. Cross-Boundary Contract Lock (Rust $\leftrightarrow$ TypeScript)

Dual-cased DTOs were defined and synchronized across Rust and TypeScript to ensure zero field drift and 100% backward compatibility:

- **TypeScript Definitions**: [`src/lib/types/champ_select.ts`](file:///C:/Users/lucas/workspace/eurmn/lcu-api/lcu-companion/src/lib/types/champ_select.ts)
  - `ActiveComboSynergy`: `entityA`, `entityB`, `entityAName`, `entityBName`, `synergyLift`, `lcbSynergyLift`, `comboWpa`, `sampleCount`, `isAugmentAugment`, `isItemAugment`, `isItemItem`.
  - `ItemRecommendation`: `itemId`, `itemName`, `slot`, `predictedWinrate`, `wpa`, `lcbWpa`, `confidenceGrade`, `sampleCount`, `itemCategory`, `reasoning`.
  - `AugmentCardRecommendation`: `augmentId`, `augmentName`, `tier`, `score`, `baseWpa`, `kitSynergyLift`, `rerollEv`, `upgradeProb`, `verdict`, `expectedRerollGain`, `activeComboPartners`, `bestLookaheadPartnerId`, `bestLookaheadPartnerName`, `bestLookaheadPartnerKind`, `bestLookaheadSynergyLift`.
  - `RerollEvRecommendation`: `tier`, `expectedValue`, `upgradeProbability`, `samplePoolSize`, `verdict`.
  - `BuildRecommendationResult`: `championId`, `championName`, `championBaselineWinrate`, `predictedBuildWinrate`, `predictedBuildWpa`, `confidenceGrade`, `itemRecommendations`, `augmentCardRecommendations`, `activeComboSynergies`, `rerollEv`, `performanceStats`.
  - `MayhemComboDto`: `comboKey`, `kind`, `championId`, `entityA`, `entityB`, `entityAName`, `entityBName`, `entityAKind`, `entityBKind`, `winRate`, `comboWpa`, `synergyLift`, `lcbSynergyLift`, `confidenceGrade`, `gamesCount`, `winsCount`.
  - `LiveGameState`: `isAugmentCheckpoint`, `augmentSlot`.
- **IPC Callers**: [`src/lib/api/champ_select.ts`](file:///C:/Users/lucas/workspace/eurmn/lcu-api/lcu-companion/src/lib/api/champ_select.ts)
  - `computeMayhemBuildRecommendation(request)` $\rightarrow$ calls `compute_mayhem_build_recommendation` with fallback to `api_compute_mayhem_build_recommendation`.
  - `queryMayhemCombos(championId, comboKind, minGames, limit)` $\rightarrow$ calls `query_mayhem_combos` with fallback to `api_query_mayhem_combos`.
- **Public API Re-export**: [`src/lib/api/agent.ts`](file:///C:/Users/lucas/workspace/eurmn/lcu-api/lcu-companion/src/lib/api/agent.ts)

---

## 3. Rust Backend & Statistical Engine (`src-tauri/src/agent/mayhem_engine.rs`)

### 3.1. Packed `u64` Bit Layout
Each entity or multi-resource interaction is encoded into a single 64-bit integer:
```rust
#[inline(always)]
pub const fn pack_combo_key(kind: u8, champ_id: u16, entity_a: u32, entity_b: u32) -> u64 {
    (((kind as u64) & 0xF) << 60)
        | (((champ_id as u64) & 0xFFFF) << 44)
        | (((entity_a as u64) & 0x3FFFFF) << 22)
        | ((entity_b as u64) & 0x3FFFFF)
}
```
Canonical combo kinds (0..=10):
- `0`: ChampBase
- `1`: AugSingle
- `2`: ItemSingle
- `3`: AugChamp
- `4`: ItemChamp
- `5`: AugAugGlobal (sorted $a \le b$)
- `6`: ItemItemGlobal (sorted $a \le b$)
- `7`: ItemAugGlobal (always $a = \text{aug}, b = \text{item}$)
- `8`: AugAugChamp (sorted $a \le b$)
- `9`: ItemItemChamp (sorted $a \le b$)
- `10`: ItemAugChamp ($a = \text{aug}, b = \text{item}$)

### 3.2. Single-Pass N-Gram Aggregation
- Queries `global-aram.db` (`aram_participants`) with fallback to `agent.db` (`game_build_facts`).
- Filters out Poro-Snax (`2050`, `2052`, `220013`).
- Parses or hashes augment strings to fit 22 bits cleanly while preserving reverse mappings.
- Emits all single, pairwise global, and 3-way champion-conditioned keys in a single pass into a `HashMap<u64, ComboStat>`.
- Aggregates augment tier and item category totals for survivorship normalization.
- Protects index via in-memory thread-safe `Arc<RwLock<Option<MayhemSynergyIndex>>>`.

### 3.3. 4-Level Hierarchical Empirical Bayes & Wilson LCB
- **Tier/Slot Normalization**: $\delta_{\text{tier}}(r) = \bar{p}_{\text{tier}}(r) - 0.50$.
- **Champion Baseline**: $p_{\text{champ}} = \frac{W_c + 30.0 \cdot 0.50}{N_c + 30.0}$.
- **Prior**: $p_0(r, c) = (p_{\text{champ}} + \delta_{\text{tier}}(r)).\text{clamp}(0.05, 0.95)$.
- **Global & Personal Blended Posterior**:
  $$p_{\text{glob}}(r \mid c) = \frac{W_{rc,\text{glob}} + 25.0 \cdot p_0(r, c)}{N_{rc,\text{glob}} + 25.0}$$
  $$p_{\text{blend}}(r \mid c) = \frac{W_{rc,\text{pers}} + 10.0 \cdot p_{\text{glob}}(r \mid c)}{N_{rc,\text{pers}} + 10.0}$$
- **Wilson LCB** ($z = 1.28155$):
  $$p_{\text{LCB}} = \text{wilson\_lcb}(\tilde{W}, \tilde{N}, 1.28155)$$

### 3.4. Pairwise & 3-Way Synergy Lift with Kit-Eligibility Gating
- Independence baseline: $p_{\text{indep}}(x, y \mid c) = (p_{\text{champ}} + \text{wpa}(x \mid c) + \text{wpa}(y \mid c)).\text{clamp}(0.05, 0.95)$.
- Global combo lift with eligibility gate: $\text{elig} = 1.0$ if $N_{xc} > 0 \land N_{yc} > 0$ else $0.0$.
- Informative prior: $p_{\text{prior}, xyc} = (p_{\text{indep}}(x, y \mid c) + 0.35 \cdot \text{elig} \cdot \delta_{\text{syn},\text{glob}}(x, y)).\text{clamp}(0.05, 0.95)$.
- Posterior combo win rate: $p_{\text{combo}}(x, y \mid c) = \frac{W_{xyc} + 15.0 \cdot p_{\text{prior}, xyc}}{N_{xyc} + 15.0}$.
- Pure synergy lift: $\Delta_{\text{syn}}(x, y \mid c) = p_{\text{combo}}(x, y \mid c) - p_{\text{indep}}(x, y \mid c)$.

### 3.5. Log-Odds Build Simulator & 3-Card Augment Recommender
- $L_0 = \text{logit}(p_{\text{champ}})$.
- Main effects weight $w_{\text{main}} = 0.75$.
- Active synergy effects weight $w_{\text{syn}} = 0.85$ with geometric decay $\gamma = 0.70^k$.
- Hyperbolic tangent saturation: $\Delta L_{\text{sat}} = 1.50 \cdot \tanh\left(\frac{\Delta L_{\text{main}} + \Delta L_{\text{syn}}}{1.50}\right)$.
- Predicted build win rate: $p_{\text{build}} = \text{sigmoid}(L_0 + \Delta L_{\text{sat}})$.
- Augment candidate scoring with slot lookahead weight ($\lambda_{\text{look}} \in [0.65, 0.45, 0.25, 0.10]$), partner unlock detection, and standard error uncertainty penalty.
- Non-repeating reroll expected value $\text{EV}_{\text{reroll}}$ over unseen pool $U_{T, c}$ with upgrade probability.

---

## 4. Tauri Commands & Live Client Poller Integration

1. **Build Engine Commands**: [`src-tauri/src/commands/build_engine.rs`](file:///C:/Users/lucas/workspace/eurmn/lcu-api/lcu-companion/src-tauri/src/commands/build_engine.rs)
   - `compute_mayhem_build_recommendation` & `api_compute_mayhem_build_recommendation`
   - `query_mayhem_combos` & `api_query_mayhem_combos`
   - Executes inside bounded `tokio::task::spawn_blocking` blocks.
2. **Command Registration**: [`src-tauri/src/lib.rs`](file:///C:/Users/lucas/workspace/eurmn/lcu-api/lcu-companion/src-tauri/src/lib.rs)
   - Registered all commands in Tauri builder invoke handlers.
3. **Live Game Poller**: [`src-tauri/src/services/live_game.rs`](file:///C:/Users/lucas/workspace/eurmn/lcu-api/lcu-companion/src-tauri/src/services/live_game.rs)
   - Reduced client timeout to 800ms to maintain real-time responsiveness.
   - Detects augment checkpoints at levels 3, 7, 11, 15 and publishes current `augment_slot`.

---

## 5. Svelte 5 Frontend Implementation

1. **`MayhemAugmentAdvisorCard.svelte`**:
   - Live 3-card augment selection view matching Mayhem visual tier colors (Silver `#a3b9bd`, Gold `#e3c49f`, Prismatic `#f2c2ff`).
   - Renders BEST PICK and REROLL verdict badges, upgrade probability, net reroll gain, and kit lift.
   - Highlights active synergy partners and lookahead partner unlocks with icon previews.
   - Includes Checkpoint banner, Command Line Reroll toggle (+1 tier simulated upgrade), and manual slot selection tabs.
2. **`MayhemCombosCard.svelte`**:
   - Fixed-row-height (68px) window virtualization rendering ~15 visible DOM rows from potentially hundreds of combinations.
   - Category filtering pills: "All Combos", "Item + Augment", "Augment + Augment", "Item + Item", "Augment + Champ", "Item + Champ", "3-Way Champion".
   - Badges displaying Combo +WPA%, Pure Synergy Lift $\Delta_{\text{syn}}$, Confidence Grade, and match count.
3. **`MayhemBuildSimulator.svelte`**:
   - Optimistic local state updates with `pending` prop and subtle pulse indicator.
4. **`MayhemChampSelectView.svelte`**:
   - 150ms debounced backend recalculation with request sequencing protection (`reqSeq`).
   - Retains `lastGoodRec` state during loading to prevent layout shifts.
   - Displays Log-Odds Build Power Bar: Champion Baseline Winrate $\to$ Predicted Build Winrate (+WPA%) + active combo $\Delta_{\text{syn}}$ pills.
   - 1-click live client game inventory sync into simulator slots.

---

## 6. Verification & Mathematical Invariants

- **Zero-Division Guards**: All denominators check $n > 0$ and fallback to sensible priors ($0.50$ or baseline).
- **Probability Bounds**: Every intermediate probability is clamped to $[0.05, 0.95]$ or $[0.01, 0.99]$, ensuring valid non-NaN logits and Wilson scores.
- **Finite Output Verification**: All returned floating-point scores, win rates, and lifts are validated with `.is_finite()`.
- **Database Safety**: Short-lived scoped connections with 5s busy timeout; no SQLite operations held across `.await` points.
- **Contract Parity**: Dual-cased DTOs ensure flawless communication across Tauri v2 IPC boundary without TypeScript property errors.
- **Trait Derives & Rust Build Hygiene**: `EvaluatedResource` derives `#[derive(Debug, Clone, Copy, PartialEq)]`, satisfying trait bounds for collection cloning, and unused imports (`PathBuf`) are pruned to keep builds warning-free.

