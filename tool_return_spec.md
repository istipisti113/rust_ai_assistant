## Tool Return Value Requirements

When calling one of the provided tool functions, the function **must** return a JSON object that matches the schema defined by the tool. The API will only accept the response if:

1. The response is a valid JSON object.
2. All required fields are present with the correct data types.
3. No extra or misspelled fields are included.
4. Values are within any documented limits (e.g. amount > 0).

Below is the schema for each function, followed by a minimal example of a valid return value.

---

### 1. `make_order`

**Input Parameters**:
```json
{
  "amount": number,
  "side": boolean,
  "symbol": string
}
```

**Required Return Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `order_id` | string | Unique identifier assigned by the trading system. |
| `symbol` | string | The symbol that was ordered. |
| `amount` | number | The amount of shares ordered. |
| `side` | boolean | `true` for buy, `false` for sell. |
| `status` | string | One of `"pending"`, `"filled"`, `"rejected"`. |
| `timestamp` | string | ISO 8601 UTC timestamp of the order creation. |

**Example**:
```json
{
  "order_id": "ORD123456",
  "symbol": "AAPL",
  "amount": 10,
  "side": true,
  "status": "pending",
  "timestamp": "2026-11-07T12:34:56Z"
}
```

---

### 2. `get_market_data`

**Input Parameters**:
```json
{
  "symbol": string
}
```

**Required Return Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `symbol` | string | The requested symbol. |
| `price` | number | Current mid‑price (float). |
| `bid` | number | Current best bid price. |
| `ask` | number | Current best ask price. |
| `volume` | number | Current day volume (int). |
| `timestamp` | string | ISO 8601 UTC timestamp of the data point. |

**Example**:
```json
{
  "symbol": "MSFT",
  "price": 299.85,
  "bid": 299.80,
  "ask": 299.90,
  "volume": 12345678,
  "timestamp": "2026-11-07T12:35:01Z"
}
```

---

### 3. `Web`

**Input Parameters**:
```json
{
  "phrase": string
}
```

**Required Return Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `results` | array of objects | Each object contains `title`, `link`, and optional `snippet`. |

**Example**:
```json
{
  "results": [
    {
      "title": "Stock Market Today",
      "link": "https://example.com/article",
      "snippet": "The market closed up 0.5%..."
    }
  ]
}
```

---

### 4. `Bash`

**Input Parameters**:
```json
{
  "command": string
}
```

**Required Return Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `stdout` | string | Standard output from the command. |
| `stderr` | string | Standard error output (may be empty). |
| `exit_code` | integer | Process exit code. |

**Example**:
```json
{
  "stdout": "Hello, world!",
  "stderr": "",
  "exit_code": 0
}
```

---

### 5. `Write`

**Input Parameters**:
```json
{
  "content": string,
  "file_path": string
}
```

**Required Return Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `status` | string | One of `"success"`, `"error"`. |
| `message` | string | Human‑readable message (e.g., "File written successfully"). |

**Example**:
```json
{
  "status": "success",
  "message": "File written successfully"
}
```

---

### 6. `Read`

**Input Parameters**:
```json
{
  "file_path": string
}
```

**Required Return Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `content` | string | File contents. |
| `status` | string | One of `"success"`, `"error"`. |
| `message` | string | Optional error message if status is `error`. |

**Example**:
```json
{
  "status": "success",
  "content": "Line1\nLine2"
}
```

---

**Important**: Any deviation from these schemas will cause the API to reject the response. For example, returning a string instead of a JSON object, omitting a required field, or adding an unknown field will result in an error. Always validate your output against the schema before returning it to the API.

If you are unsure about any field, feel free to ask for clarification!