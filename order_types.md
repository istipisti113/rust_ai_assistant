# Alpaca Order Types Explained

Alpaca supports five core equity order types. Each has a distinct **trigger** and a distinct **execution mechanism** after that trigger. The essential question for every order type is: *what causes it to fire, and what price protection do you get once it fires?*

## Quick Reference

| Order Type (`type`) | What triggers it | Execution after trigger | Required parameter(s) |
| :--- | :--- | :--- | :--- |
| **`market`** | Immediately upon submission | Best available current price | None beyond `symbol`, `side`, `qty` |
| **`limit`** | Market reaches your price or better | At your limit price or better (never worse) | `limit_price` |
| **`stop`** (stop-market) | Price crosses your `stop_price` | Becomes a **market order** — no price guarantee | `stop_price` |
| **`stop_limit`** | Price crosses your `stop_price` | Becomes a **limit order** at your `limit_price` | `stop_price` **and** `limit_price` |
| **`trailing_stop`** | Price retraces from a moving high-water mark by your trail amount | Becomes a **market order** | one of `trail_price` or `trail_percent` |

---

## Market Order

A market order is a request to buy or sell at the **currently available market price**. It is the most likely method to get filled and fills nearly instantaneously.

**Trade-off — price slippage:** your fill price can differ from the last quoted price depending on available liquidity and price movement while the order is routed. Short-term price spikes can cause fills at unexpected prices.

Requires no price parameter.

---

## Limit Order

A limit order sets the **worst price you are willing to accept**.

- A **buy** limit executes at the limit price or **lower**.
- A **sell** limit executes at the limit price or **higher**.

You must supply `limit_price`.

**Pros**
- Prevents slippage — you never pay more (buy) or receive less (sell) than your limit.

**Cons**
- May not fill at all if the market never reaches your price.
- May sit unfilled for a long time.
- If the market is exactly at your limit price, the order fills only if resting liquidity is available to trade against.

**Sub-penny rule:** Alpaca enforces sub-penny restrictions on `limit_price` — at or above `$1.00`, a maximum of 2 decimals; below `$1.00`, a maximum of 4 decimals.

---

## Stop Order (Stop-Market)

A stop order is a **conditional market order**. It lies dormant until the price trades through your `stop_price`, at which point it is **elected and becomes a market order**.

- You must supply `stop_price`.
- **Critical caveat:** there is **no price guarantee** after election. The stop price is only the trigger; the actual fill can be materially worse, especially in fast markets or gaps.
- Sell stop orders are *not* automatically converted into stop-limit orders.

**Example:** TSLA trading at $200; a buy stop at $210 remains dormant until TSLA reaches $210, then fires as a market buy.

---

## Stop-Limit Order

A stop-limit order combines a **stop trigger** with a **limit price constraint**. Once `stop_price` is reached, the order does **not** become a market order — it becomes a **limit order** at your `limit_price` and remains active until it fills at that price or better, or you cancel it.

- You must supply **both** `stop_price` and `limit_price`.

**Pros**
- You control the worst execution price after the trigger.

**Cons**
- If the market gaps through both your stop and your limit, the limit may **not fill at all**, leaving you with an open position you intended to exit.

**Example:** buy stop-limit with stop `$210` and limit `$215` — if TSLA reaches $210, the order triggers and becomes a limit buy at $215, executing only at $215 or better.

---

## Trailing Stop Order

A trailing stop is a **dynamic stop** that follows the market in your favor. Instead of a fixed stop price, you specify a **trail amount** — either a dollar offset (`trail_price`) or a percentage offset (`trail_percent`) — away from the **high-water mark (HWM)**.

- **Sell** trailing stop: the HWM is the **highest** price seen since submission. The stop price is `HWM − trail`. If the stock rises, the stop rises with it; if the stock falls, the stop stays put until it is hit.
- **Buy** trailing stop: the logic is mirrored around the **lowest** price seen.
- **Exactly one** of `trail_price` or `trail_percent` is required when `type` is `"trailing_stop"`.

**Examples**
- `trail_price: "2.00"` on a sell → stop price is always `HWM − 2.00`.
- `trail_percent: "1.0"` on a sell → stop price is always `HWM × 0.99` (1% below the high-water mark).

Once the derived stop price is triggered, the order **becomes a market order**, so the actual fill may be above or below the trigger price.

**Additional constraints**
- Trailing stops **will not trigger outside regular market hours**.
- Valid `time_in_force` values are only **`day`** and **`gtc`**.
- Currently supported only as **single orders** — not yet as the stop-loss leg of a bracket or OCO order.

