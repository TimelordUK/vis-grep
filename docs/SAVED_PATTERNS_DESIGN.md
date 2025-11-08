# Saved Patterns & Query Language Design

## Problem Statement

When searching FIX messages and logs, users need:

1. **Saved Regex Patterns**: Complex regex like `\x01150=H\x01` (with SOH markers) or conditional patterns like "find 71=2 only in messages where 35=8 or 35=J"
2. **Simple Query Language**: For non-regex users, a way to express AND/OR/NOT without writing regex

## Proposed Solution

### Part 1: Saved Regex Patterns in Config

Add to `~/.config/vis-grep/config.yaml`:

```yaml
folder_presets:
  - name: "Logs"
    path: "~/logs"

saved_patterns:
  - name: "FIX: Exec Status 150=H"
    pattern: "\x01150=H\x01"
    description: "Order status Held"

  - name: "FIX: Order Cancel Reject with reason"
    pattern: "35=9.*\x0171=2"
    description: "Find 71=2 (Unknown order) in order cancel reject"

  - name: "FIX: Execution Report or Execution Ack with Manual fill"
    pattern: "(35=8|35=J).*\x0171=2"
    description: "71=2 in exec reports or acks only"

  - name: "FIX: ClOrdID pattern"
    pattern: "\x0111=[A-Z0-9]{10,}\x01"
    description: "ClOrdID with 10+ alphanumeric chars"

  - name: "Error with stack trace"
    pattern: "(?m)ERROR.*\\n\\s+at "
    description: "Error messages with Java-style stack traces"
```

**UI Integration:**
- Dropdown next to query field labeled "📝 Patterns"
- Select a pattern to auto-fill the search query
- Show description on hover
- Button to edit config file

### Part 2: Simple Query Language (Alternative to Regex)

For simpler cases, support a query syntax:

```
Basic AND (implicit):
  150=H 35=8        → matches lines with both

Explicit AND:
  150=H AND 35=8    → same as above

OR:
  35=8 OR 35=J      → matches either

NOT:
  ERROR NOT timeout → has ERROR but not timeout

Grouping:
  (35=8 OR 35=J) AND 71=2
  → 71=2 in exec report OR exec ack

Phrases:
  "order rejected"  → exact phrase

Field matching (FIX-aware):
  tag:35=8          → specifically in tag 35
  tag:150=H         → specifically tag 150
```

**Implementation Options:**

1. **Use `search-query-parser` crate**
   - Parses complex queries with AND/OR/NOT
   - Supports parentheses and phrases
   - Mature and well-tested

2. **Custom simple parser**
   - Parse basic AND/OR/NOT
   - Convert to regex internally
   - Lighter weight

**UI Toggle:**
- Radio buttons: `[●] Simple Query  [ ] Regex`
- When Simple Query mode:
  - Parse query into boolean logic
  - Convert to regex or apply filters
  - Show syntax help
- When Regex mode:
  - Direct regex matching (current behavior)

### Part 3: FIX-Aware Features (Future)

For FIX messages specifically:

```yaml
fix_patterns:
  - name: "Held Orders"
    description: "Find orders with status Held (150=H)"
    tags:
      - tag: 150
        value: "H"

  - name: "Manual Fill in ExecReport"
    description: "Manual OrderOrigination in execution reports"
    conditions:
      - tag: 35
        value: ["8", "J"]  # ExecReport or ExecutionAcknowledgement
      - tag: 71
        value: "2"         # Manual

  - name: "Rejected New Orders"
    tags:
      - tag: 35
        value: "8"
      - tag: 39
        value: "8"  # Rejected
      - tag: 150
        value: "8"  # Rejected
```

This would:
- Understand SOH delimiters automatically
- Parse FIX tags properly
- Handle tag ordering issues
- Support tag value lists

## Implementation Plan

### Phase 1: Saved Regex Patterns (Quick Win)
1. Add `saved_patterns` to Config struct
2. Add dropdown UI next to search query
3. Click pattern → fills search query field
4. Show description in tooltip

**Complexity**: Low
**Value**: High for power users

### Phase 2: Simple Query Language
1. Add dependency: `search-query-parser = "0.1"`
2. Add query mode toggle (Simple/Regex)
3. Parse simple queries to search logic
4. Apply boolean logic to results

**Complexity**: Medium
**Value**: High for non-regex users

### Phase 3: FIX-Aware Patterns (Advanced)
1. Add FIX message parser
2. Understand tag structure
3. Handle SOH automatically
4. Structured tag matching

**Complexity**: High
**Value**: Very high for FIX-specific use case

## Examples

### Current Way (Regex only):
```
Search: \x0135=(8|J)\x01.*\x0171=2\x01
```
Hard to read, easy to get wrong.

### With Saved Patterns:
```
Dropdown → "Manual Fill in ExecReport"
Auto-fills regex, user clicks search
```

### With Simple Query:
```
Mode: Simple Query
Query: (35=8 OR 35=J) AND 71=2
```
Tool converts to appropriate regex or applies logic.

### With FIX-Aware (future):
```
Mode: FIX Query
35 in [8, J] AND 71 = 2
```
Automatically handles SOH, tag ordering, etc.

## Config Example

```yaml
folder_presets:
  - name: "FIX Logs"
    path: "~/work/fix-logs"

saved_patterns:
  # FIX patterns
  - name: "Order Status Held"
    pattern: "\x01150=H\x01"
    description: "OrdStatus = Held (150=H)"
    category: "FIX"

  - name: "Execution Report"
    pattern: "\x0135=8\x01"
    description: "Message type = Execution Report"
    category: "FIX"

  - name: "Manual OrderOrigination in Exec"
    pattern: "35=8.*71=2"
    description: "Find OrderOrigination=Manual (71=2) in execution reports"
    category: "FIX"

  # Error patterns
  - name: "Java Exception"
    pattern: "(?m)Exception.*\\n\\s+at "
    description: "Java exceptions with stack traces"
    category: "Errors"

  - name: "Critical Errors"
    pattern: "(CRITICAL|FATAL|SEVERE)"
    description: "High-severity log messages"
    category: "Errors"

  # Generic patterns
  - name: "IP Address"
    pattern: "\\b(?:[0-9]{1,3}\\.){3}[0-9]{1,3}\\b"
    description: "IPv4 addresses"
    category: "Network"

  - name: "UUID"
    pattern: "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
    description: "Standard UUID format"
    category: "Generic"
```

## UI Mockup

```
┌─────────────────────────────────────────────────────────────┐
│ Search Query:                                                │
│ ┌─────────────────────────────────────┐  ┌────────────────┐ │
│ │ \x0135=8\x01.*\x0171=2\x01          │  │ 📝 Patterns ▼ │ │
│ └─────────────────────────────────────┘  └────────────────┘ │
│                                                               │
│ Mode: ( ) Simple Query  (●) Regex                           │
│                                                               │
│ When dropdown clicked:                                       │
│ ┌────────────────────────────────────────┐                  │
│ │ FIX ──────────────────────────────────│                  │
│ │   Order Status Held                    │                  │
│ │   Execution Report                     │                  │
│ │   Manual OrderOrigination in Exec      │ ← shows desc    │
│ │ Errors ────────────────────────────────│                  │
│ │   Java Exception                       │                  │
│ │   Critical Errors                      │                  │
│ │ Generic ───────────────────────────────│                  │
│ │   IP Address                           │                  │
│ │   UUID                                 │                  │
│ └────────────────────────────────────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

## Benefits

1. **For FIX Users**: Pre-made patterns for common searches, no need to remember SOH encoding
2. **For Non-Regex Users**: Simple AND/OR/NOT query syntax
3. **For Power Users**: Full regex still available
4. **Team Sharing**: Config file can be version controlled and shared
5. **Documentation**: Pattern descriptions serve as documentation

## Open Questions

1. Should we support editing patterns in the UI or just via config file?
2. Should simple query mode convert to regex or apply post-search filtering?
3. Do we need FIX-aware parsing or are regex patterns + simple queries enough?
4. Should patterns support variables? e.g., `tag:35=$MSGTYPE`

## Next Steps

1. ✅ Document the design (this file)
2. Implement Phase 1 (saved regex patterns)
3. Evaluate Phase 2 (simple query language) - may not be needed if patterns cover most cases
4. Consider Phase 3 (FIX-aware) based on user feedback
