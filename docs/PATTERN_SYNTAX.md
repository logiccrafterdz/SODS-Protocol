# Advanced Pattern Syntax

## Advanced Pattern Syntax

### Quantifiers
- `{n}`: Exactly n occurrences
- `{n,}`: At least n occurrences  
- `{n,m}`: Between n and m occurrences (greedy consumption)

### Examples
```bash
# Detect sandwich attacks with 2-5 swaps
sods verify "Tf -> Sw{2,5} -> Tf"

# Monitor large transfers in last hour
sods verify "Tf where value > 1000 ether" --time-window 3600
```

### Limitations
- Maximum symbols per pattern: 10 (to prevent ReDoS)
- Nested quantifiers not supported
- Complex conditions require predefined patterns
