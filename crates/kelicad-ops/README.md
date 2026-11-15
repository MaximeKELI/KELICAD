# kelicad-ops

Atomic `Operation`s that mutate `kelicad-doc::Document`, an immutable journal,
and undo/redo. No direct document writes should happen outside this crate.
