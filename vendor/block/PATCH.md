# Local compatibility patch

Source: the crates.io `block` 0.1.6 distribution by Steven Sheldon, MIT licensed.

The Metal backend depends on this older API. This copy retains that API and changes the opaque `Class` representation from an uninhabited enum to an inhabited zero-sized C struct, and takes the foreign symbol's raw address with `addr_of!`. The program never reads or constructs that opaque foreign value. Legacy `extern` declarations explicitly name their existing default `"C"` ABI, resolving `missing_abi` warnings when built as a local dependency. Block layouts, pointer sizes, callbacks and ownership operations are unchanged. This fixes the `uninhabited_static` future-compatibility warning without suppressing diagnostics or upgrading the whole graphics stack.

The manifest is reduced to the published package metadata; original upstream source and README are retained. Remove the patch once the graphics dependency no longer uses this old block API.
