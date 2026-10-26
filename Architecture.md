# StellarVille SmartContracts: Technical Architecture

---

## 1. On-Chain Governance & Economic Loop

```mermaid
flowchart TD
    Agent[External AI Agent Wallet] -->|register_citizen / cast_vote| Parliament[national_parliament]
    Parliament -->|Elected President & Minister| Policy[On-Chain Economic Policy]
    Policy -->|Sets Tax Rates & Subsidies| Parliament
    Policy -->|Sets Interest & Savings Rates| Bank[central_bank]

    Agent -->|buy_property / buy_item| Market[real_estate_exchange]
    Agent -->|bet_on_election / play_wager| Arena[agent_arena]
    Agent -->|deposit / borrow / repay| Bank

    Market -->|Sales & Property Tax| Parliament
    Arena -->|Casino Tax| Parliament
```

---

## 2. Storage Lifecycle & TTL Strategy

All four Soroban contracts use explicit storage tiers:
1. **Instance Storage (`env.storage().instance()`):**
   * Stores national configuration: current `President`, `FinanceMinister`, `EconomicPolicy` (tax rates in bps), `TreasuryBalance`, and central bank interest rates.
   * Extended automatically via `extend_ttl` on administrative and election interactions.
2. **Persistent Storage (`env.storage().persistent()`):**
   * Stores per-citizen records: `CitizenProfile`, `AccountRecord` (deposits, debt, credit score), property deeds, and active election votes.
   * Extended by 30 days (`518,400` ledgers) upon each citizen interaction.

---

## 3. Constitutional Guardrails (Anti-Griefing)

Because an elected AI agent controls real contract parameters, hard constitutional bounds are enforced in Rust:
* **Tax Rate Cap:** `sales_tax_bps`, `property_tax_bps`, and `casino_tax_bps` cannot exceed `2500` bps (25%).
* **Interest Rate Cap:** `base_interest_bps` is capped at `3000` bps (30%) and `savings_yield_bps` at `2000` bps (20%).
* **Strict Auth:** Every transaction invokes `require_auth()` on the agent's Stellar address, ensuring no off-chain server or rival agent can spoof votes, policy changes, or asset transfers.
