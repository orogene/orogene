# Benchmarks

Even at this early stage, orogene is **very** fast. These benchmarks are
all on ubuntu linux running under wsl2, with an ext4 filesystem.

All benchmarks are ordered from fastest to slowest (lower is better):

## Warm Cache

This test shows performance when running off a warm cache, with an
existing lockfile. This scenario is common in CI scenarios with caching
enabled, as well as local scenarios where `node_modules` is wiped out in
order to "start over" (and potentially when switching branches).

Of note here is the contrast between the subsecond (!) installation by
orogene, versus the much more noticeable install times of literally
everything else.

| Package Manager | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `orogene` | 417.3 ± 43.3 | 374.6 | 524.8 | 1.00 |
| `bun` | 1535.2 ± 72.5 | 1442.3 | 1628.9 | 3.68 ± 0.42 |
| `pnpm` | 8285.1 ± 529.0 | 7680.4 | 9169.9 | 19.85 ± 2.42 |
| `yarn` | 20616.7 ± 1726.5 | 18928.6 | 24401.5 | 49.41 ± 6.59 |
| `npm` | 29132.0 ± 4569.2 | 25113.4 | 38634.2 | 69.81 ± 13.13 |

## Cold Cache

This test shows performance when running off a cold cache, but with an
existing lockfile. This scenario is common in CI scenarios that don't
cache the package manager caches between runs, and for initial installs by
teammates on relatively "clean" machines.

| Package Manager | Mean [s] | Min [s] | Max [s] | Relative |
|:---|---:|---:|---:|---:|
| `bun` | 5.203 ± 1.926 | 3.555 | 9.616 | 1.00 |
| `orogene` | 8.346 ± 0.416 | 7.938 | 9.135 | 1.60 ± 0.60 |
| `pnpm` | 27.653 ± 0.467 | 26.915 | 28.294 | 5.31 ± 1.97 |
| `npm` | 31.613 ± 0.464 | 30.930 | 32.192 | 6.08 ± 2.25 |
| `yarn` | 72.815 ± 1.285 | 71.275 | 74.932 | 13.99 ± 5.19 |

## Memory Usage

Another big advantage of Orogene is significantly lower memory usage
compared to other package managers, with each scenario below showing the
peak memory usage (resident set size) for each scenario (collected with
/usr/bin/time -v):

| Package Manager | no lockfile, no cache | lockfile, cold cache | lockfile, warm cache | existing node_modules |
|:---|---:|----:|---:|----:|
| `orogene` | 266.8 mb | 155.2 mb | 38.6 mb | 35.5 mb |
| `bun` | 2,708.7 mb | 792.1 mb | 34.5 mb | 25.8 mb |
| `pnpm` | 950.9 mb | 638.4 mb | 260.1 mb | 168.7 mb |
| `npm` | 1,048.9 mb | 448.2 mb | 833.7 mb | 121.7 mb |
| `yarn` | 751.1 mb | 334.4 mb | 251.9 mb | 129.3 mb |

## Caveat Emptor

At the speeds at which orogene operates, these benchmarks can vary widely
because they depend on the underlying filesystem's performance. For
example, the gaps might be much smaller on Windows or (sometimes) macOS.
They may even vary between different filesystems on Linux/FreeBSD. Note
that orogene uses different installation strategies based on support for
e.g. reflinking (btrfs, APFS, xfs).
