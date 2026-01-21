# SODS Use Cases

Beyond simple verification, SODS can be used for proactive behavioral analysis.

## Use Case: Detecting Rug Pulls Early

Monitoring liquidity removal events (`LP-`) can give early warning of potential rug pulls or massive exits.

1.  **Run Discovery**: Scan for recent liquidity removals.
    ```bash
    sods discover --symbol LP- --chain base --last 100
    ```

2.  **Analyze**:
    *   If a block has **> 5 `LP-` events** and **0 `Sw` (Swap) events**, it indicates liquidity is being pulled without trading activity.
    *   *Action*: Investigate the contracts involved in that block.

## Use Case: Finding MEV Opportunities

High densities of swaps in a single block often indicate high volatility or arbitrage opportunities (MEV).

1.  **Run Discovery**: Scan for high swap activity.
    ```bash
    sods discover --symbol Sw --chain arbitrum --last 50
    ```

2.  **Analyze**:
    *   Blocks with **abnormally high swap counts** (e.g., > 100) are potential sandwich attack targets or arbitrage battlegrounds.
    *   *Action*: Deep dive into verified specific blocks to analyze the flow.
