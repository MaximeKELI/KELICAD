# kelicad-constraints

2D parametric constraints (coincidence, parallel, perpendicular, equal length,
horizontal/vertical, driving distance). The solver returns `Operation` batches
and never mutates `Document` directly (ADR 0010).

```bash
cargo test -p kelicad-constraints
```
