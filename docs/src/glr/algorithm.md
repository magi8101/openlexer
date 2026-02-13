# GLR Algorithm

This chapter describes the GLR parsing algorithm implemented in OpenLexer.

## Data Structures

### Graph-Structured Stack (GSS)

The GSS is a directed acyclic graph where:
- Nodes represent parser states
- Edges represent stack entries (symbols and semantic values)

```rust
struct GssNode {
    state: usize,
    links: Vec<GssEdge>,
}

struct GssEdge {
    predecessor: NodeId,
    symbol: Symbol,
    value: SemanticValue,
}
```

### GSS Operations

**Push**: Add a new node with an edge to the current node.

**Reduce**: Follow edges backward by the rule length, then push the result.

**Merge**: When two paths reach the same state, merge them instead of duplicating.

### Shared Packed Parse Forest (SPPF)

The SPPF compactly represents all parse trees:

```rust
enum SppfNode {
    Symbol {
        symbol: String,
        span: (usize, usize),
    },
    Packed {
        rule: usize,
        children: Vec<NodeId>,
    },
    Ambiguous {
        alternatives: Vec<NodeId>,
    },
}
```

## Algorithm Steps

### 1. Initialization

Create the initial GSS node with state 0.

```
GSS: [state 0]
Active: { node_0 }
```

### 2. Per-Token Processing

For each input token:

```
pending_shifts = {}
pending_reductions = {}

for each active node:
    action = get_action(node.state, token)
    if action contains SHIFT:
        add to pending_shifts
    if action contains REDUCE:
        add to pending_reductions
```

### 3. Reduction Phase

Process all pending reductions:

```
while pending_reductions not empty:
    (node, rule) = pop from pending_reductions
    
    for each path of length rule.rhs_len from node:
        base_node = end of path
        goto_state = goto[base_node.state, rule.lhs]
        
        if node with goto_state exists in GSS:
            merge edge into existing node
        else:
            create new node with goto_state
        
        check for new reductions from the new/merged node
```

### 4. Shift Phase

Process all pending shifts:

```
new_active = {}

for each (node, shift_state) in pending_shifts:
    if node with shift_state exists:
        merge
    else:
        create new node
    
    add to new_active

active = new_active
```

### 5. Termination

After processing all tokens:
- If any active node can accept, parsing succeeds
- If no active nodes remain, parsing fails

## Conflict Actions

The `glr_conflict_actions` table stores all actions at conflict points:

```rust
struct ParsingTable {
    // Standard LALR action table (winner of conflicts)
    action: HashMap<(usize, String), Action>,
    
    // All actions at conflicts (for GLR)
    glr_conflict_actions: HashMap<usize, HashMap<String, Vec<Action>>>,
}
```

During GLR parsing, both actions are explored when a conflict exists.

## Path Enumeration

Finding all paths of length N in the GSS:

```rust
fn enumerate_paths(node: NodeId, length: usize) -> Vec<Path> {
    if length == 0 {
        return vec![Path::new(node)];
    }
    
    let mut paths = vec![];
    for edge in gss[node].links {
        for subpath in enumerate_paths(edge.predecessor, length - 1) {
            paths.push(subpath.prepend(edge));
        }
    }
    paths
}
```

## Complexity

- Time: O(n^3) worst case, O(n) for unambiguous input
- Space: O(n^2) for GSS, O(n^3) for full SPPF

The practical performance depends on the degree of ambiguity in the grammar and input.
