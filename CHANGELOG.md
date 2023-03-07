# Orogene Release Changelog

<a name="0.3.7"></a>
## 0.3.7 (2023-03-07)

This release is also the first one to include a prebuilt macOS arm64/m1 binary.

### Features

* **log:** write verbose trace to a separate debug logfile ([62df490e](https://github.com/orogene/orogene/commit/62df490e33bf4c68d1a57b5ba345c2fadbc783ac))


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

