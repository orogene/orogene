# Orogene Release Changelog

<a name="0.3.10"></a>
## 0.3.10 (2023-03-13)

### Features

* **restore:** improvements to restore command, and removal of resolve command ([4e9940dd](https://github.com/orogene/orogene/commit/4e9940dd04e632ecbf5c7a1b2068b66638e80824))
* **restore:** print timings in seconds ([608d3f59](https://github.com/orogene/orogene/commit/608d3f599bbaf8591193a917157f243ae81dab16))
* **memory:** significantly reduce memory use during resolution (#203) ([f7fb85d6](https://github.com/orogene/orogene/commit/f7fb85d60a5839b218916d8d54a331f390527716))
* **hax:** offer hackerish words of encouragement! ([cf467da4](https://github.com/orogene/orogene/commit/cf467da40e2b9daa8762d79a0c96d516b0447388))

### Bug Fixes

* **progress:** only show one progress bar at a time ([e36356c6](https://github.com/orogene/orogene/commit/e36356c64c61ccb5b4fd43c9d252578a36855362))


<a name="0.3.9"></a>
## 0.3.9 (2023-03-12)

### Features

* **wasm:** get nassun and node-maintainer working well in wasm (#131) ([16ad5bae](https://github.com/orogene/orogene/commit/16ad5bae83d15155571464c5dfca1c7de3544057))
* **validate:** optionally validate cache contents during extraction (#197) ([0e22a5f4](https://github.com/orogene/orogene/commit/0e22a5f44d02423b9d4b49fe88254ae8bd90a699))
* **extract:** remove existing modules as needed ([d3303b00](https://github.com/orogene/orogene/commit/d3303b007301fb668db3108af6d0ebd6dae7e7bf))
* **prune:** check for an prune extraneous packages, and skip extracting valid ones (#200) ([544a2c5c](https://github.com/orogene/orogene/commit/544a2c5c3065041f351aeba46506e725eb6a769a))
* **progress:** refactored progress bar out of node-maintainer (#201) ([e1908ad6](https://github.com/orogene/orogene/commit/e1908ad6bfa248b82b99fdf3bd75f2f7dff6d9a4))
* **progresss:** add flags to disable progress bars ([f988a824](https://github.com/orogene/orogene/commit/f988a824a9202080ba7d592be67e04a8c11472ee))

### Bug Fixes

* **nassun:** use cfg_attr to reduce duplication ([f126d5ca](https://github.com/orogene/orogene/commit/f126d5ca0d32d76b35d93e65acda60d86e152852))

<a name="0.3.8"></a>
## 0.3.8 (2023-03-09)

### Bug Fixes

* **reflink:** move reflink checks up to node-maintainer (#195) ([9506edc7](https://github.com/orogene/orogene/commit/9506edc7456eefb826aaa3850873f615be09136f))

### Features

* **log:** write verbose trace to a separate debug logfile (#192) ([8c995125](https://github.com/orogene/orogene/commit/8c995125e9d142547e8eadb712473d0cb09d9b36))
* **log:** log a bit more detail about lack of reflink support ([545dff0c](https://github.com/orogene/orogene/commit/545dff0c9b82b69a663117ce1bdbb91214682ee2))
* **docs:** add initial benchmark tables ([2bbd2616](https://github.com/orogene/orogene/commit/2bbd2616ed592486450d134dc4e8208b5de0a0a0))


<a name="0.3.6"></a>
## 0.3.6 (2023-03-07)

### Features

* **cow:** prefer CoW on systems that support it. Also, fall back to copy when hard links fail. ([0e29632a](https://github.com/orogene/orogene/commit/0e29632a84fe21c83dc32ad7111bbef78f2789f0))


<a name="0.3.5"></a>
## 0.3.5 (2023-03-06)

### Bug Fixes

* **tests:** remove debug leftover (#190) ([45ab7738](https://github.com/orogene/orogene/commit/45ab7738c8c0d7c4c223e29aa69bc717faea5f4c))
* **extract:** need to rewind NamedTempFile before extraction ([eb8c0af5](https://github.com/orogene/orogene/commit/eb8c0af5222efb88e236c9d68b720f1a3a42ada4))


<a name="0.3.4"></a>
## 0.3.4 (2023-03-06)

### Bug Fixes

* **error:** make IoErrors during extraction less vague ([318fd2d2](https://github.com/orogene/orogene/commit/318fd2d288353f22c18fddd8cfa8e9433acc1eb3))

<a name="0.3.3"></a>
## 0.3.3 (2023-03-06)

### Bug Fixes

* **progress:** extraction should be based on node count (minus root node) ([088da295](https://github.com/orogene/orogene/commit/088da2951ac98afeaf98d817cd25557de446c764))
* **extract:** extract_to should only be available on non-wasm ([f8792adc](https://github.com/orogene/orogene/commit/f8792adcde6b55998347d9aa858039775a901614))

### Features

* **deprecated:** Warn when resolving npm packages marked as `deprecated` (#184) ([45a953b0](https://github.com/orogene/orogene/commit/45a953b04b8301f4a280be7cd82d6597fe2d40a3))
* **maintainer:** export iterator over graph modules (#187) ([fa109bf4](https://github.com/orogene/orogene/commit/fa109bf4eb2448a56ffc86ccfae54e4838b77230))
* **maintainer:** s/parallelism/concurrency/g ([17e1fb49](https://github.com/orogene/orogene/commit/17e1fb49685aee4ccfce71e0c1ea455d548989d9))
* **modules:** wrap iterator in its own type ([ee99bee4](https://github.com/orogene/orogene/commit/ee99bee47ae2d240f9ff904e8f7208860486ad66))
* **extraction:** add support for faster, cached extraction (#191) ([5bf0425b](https://github.com/orogene/orogene/commit/5bf0425b56daadfc34ca47c71bedee814913fdc5))


<a name="0.2.1"></a>
## 0.2.1 (2023-02-26)

No changes. Just getting the release system working.

<a name="0.2.0"></a>
## 0.2.0 (2023-02-26)

This is an initial "release" mostly to make sure the release workflow is all
working, and just to get the current prototype out there and available for
people to poke at.

### Features

* **cli:** Add resolve & link progress bars (#145) ([dd4c6ca2](https://github.com/orogene/orogene/commit/dd4c6ca2f6ef441903d479bcad36d09c86f28612))

