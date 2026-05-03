# AutoFDO Investigation

Date: 2026-05-03

## Verdict

AutoFDO is viable on this machine, but it should be treated as an advanced native build mode, not the default portable installer path.

Stable `rustc` supports instrumentation PGO through `-Cprofile-generate` and `-Cprofile-use`. Sample-profile AutoFDO is still behind unstable `-Z` flags:

- `-Zdebug-info-for-profiling`
- `-Zprofile-sample-use=<profile>`

Use a nightly toolchain for production use of those flags. `RUSTC_BOOTSTRAP=1` can prove feasibility locally, but should not be the supported build interface.

## Local Toolchain Findings

- CPU target: `native` resolves to `alderlake`.
- `rustc`: 1.90.0, LLVM 20.1.8.
- System LLVM tools: 21.1.8.
- `perf` is installed and can record as root.
- `llvm-profgen` is installed and works for sample-profile generation.
- `create_llvm_prof` is installed but unusable because it was built without LLVM support.

## Minimal Working AutoFDO Flow

1. Build a profiling binary:

   ```bash
   RUSTFLAGS="-Ctarget-cpu=native -Cforce-frame-pointers=yes -Cdebuginfo=line-tables-only -Zdebug-info-for-profiling" \
     cargo +nightly build --release --target=x86_64-unknown-linux-gnu
   ```

2. Collect branch-stack samples:

   ```bash
   perf record -b -o memvid-context.perf.data \
     ./target/x86_64-unknown-linux-gnu/release/memvid-context \
     --project global --query "installer protocol recall" --budget-tokens 4000
   ```

3. Convert samples:

   ```bash
   llvm-profgen \
     --binary=./target/x86_64-unknown-linux-gnu/release/memvid-context \
     --perfdata=memvid-context.perf.data \
     --output=memvid-context.prof
   ```

4. Build with the sample profile:

   ```bash
   RUSTFLAGS="-Ctarget-cpu=native -Zprofile-sample-use=$PWD/memvid-context.prof" \
     cargo +nightly build --release --target=x86_64-unknown-linux-gnu
   ```

## Recommended Production Path

Add three build modes:

- `portable`: generic x86-64 glibc release binaries for the self-extracting installer.
- `native`: `-Ctarget-cpu=native`, LTO, low codegen units, stripped symbols.
- `native-pgo`: stable instrumentation PGO first, because it is supported by stable `rustc`.

Add AutoFDO after that as `native-autofdo`, gated on nightly Rust, root/perf access, and working `llvm-profgen`.

## Expected Value

AutoFDO is most likely to help CPU-bound tools:

- `memvid-context`
- `memvid-migrator`
- search/ranking paths in `memvid-core`

It is less likely to materially improve `memvid-embedder` end-to-end throughput because ONNX Runtime CUDA dominates embedding time. CPU-side tokenization and batching may still benefit.

## Risks

- Requires nightly or unsupported `RUSTC_BOOTSTRAP`.
- Profiles are workload-sensitive and can become stale.
- `perf` permissions vary by distro and kernel policy.
- Applying one merged profile workspace-wide can produce missing-profile warnings; per-binary profiles are cleaner.
- Sample-profile support should not replace stable instrumentation PGO until benchmarked.
