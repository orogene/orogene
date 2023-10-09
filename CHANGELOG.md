# Orogene Release Changelog
<a name="0.3.34"></a>
## 0.3.34 (2023-10-09)

This release is largely a series of attempts at making Orogene faster, most of
which didn't have much of an effect. Properly supporting reflinks on DeDrive
is nice, though.

### Features

* **reflink:** bump reflink-copy to add support for reflinks on Windows Dev Drive ([32c169c7](https://github.com/orogene/orogene/commit/32c169c71aaf252acf14a79bfd776d5e1e82d022))
* **perf:** be smarter about reflinks and mkdirp ([581dda60](https://github.com/orogene/orogene/commit/581dda60f931c6a474a283f6904e496d9c7c6d98))

### Bug Fixes

* **setup:** fix issue with repeatedly being asked about telemetry ([a0111278](https://github.com/orogene/orogene/commit/a0111278a6398d2314a6c37c19c503dad6d78435))
* **perf:** reduce number of syscalls a bit ([3f005c1f](https://github.com/orogene/orogene/commit/3f005c1f862848c9a20176ab8ece5e2056ba61c9))

<a name="0.3.33"></a>
## 0.3.33 (2023-10-02)

### Features

* **terminal:** display indeterminate progress while orogene is running ([d4f3e845](https://github.com/orogene/orogene/commit/d4f3e8450afdc5cc5481daeb9477efd46153f066))
* **release:** automatically bump official brew formula on release ([deceec38](https://github.com/orogene/orogene/commit/deceec38b33f8b6b327bf10f00105a0531ce70db))

### Bug Fixes

* **manifest:** handle boolean bundledDependencies values ([d6c16a44](https://github.com/orogene/orogene/commit/d6c16a445fd7f3a58bf66a61c58048de2062aef8))
* **fmt:** cargo fmt --all ([6fd8b279](https://github.com/orogene/orogene/commit/6fd8b2792922898862cbcae5e0b9b45ff2aab4b2))
* **bin:** fix bin linking for the isolated linker and fix cmd shims ([8b58caba](https://github.com/orogene/orogene/commit/8b58caba8e34bb031fd8f4e37891c5d136ea8ace))
* **docs:** fix broken links ([e182ce76](https://github.com/orogene/orogene/commit/e182ce764a8c2952c1995b334e6d0a3874d0b129))
* **terminal:** don't print out terminal progress codes unless stderr is a terminal ([1228ebea](https://github.com/orogene/orogene/commit/1228ebea3d4612b45c9caa30c7536d0032fa0072))

<a name="0.3.32"></a>
## 0.3.32 (2023-09-30)

### Features

* **dist:** automatically publish to a homebrew tap ([023ebcca](https://github.com/orogene/orogene/commit/023ebcca7d2b859b5c798535eaef16556f909341))

### Bug Fixes

* **cache:** cache recovery was working off JSON metadata, not new rkyv-based one ([7c566287](https://github.com/orogene/orogene/commit/7c56628731581a225890946aa6b292f0cf6f3ec9))
* **unsafe:** use safe version of method that verifies data. No perf impact noticeable ([1a6b54ac](https://github.com/orogene/orogene/commit/1a6b54ac34c9b03ab1eab069d464c5475fa2adeb))
* **resolver:** fix issue with aliases not being handled properly ([85d31713](https://github.com/orogene/orogene/commit/85d3171300375ae3799294f3ff759ad947aa8443))
* **resolver:** node paths should use their fs name ([544639b7](https://github.com/orogene/orogene/commit/544639b7685a2738d1c2999b1ee10eb125236d23))
* **resolver:** go ahead and try to resolve if we just have a version ([8d04d84b](https://github.com/orogene/orogene/commit/8d04d84b245ccd4599964c445c58193edf089664))

### Documentation

* **build:** document build requirements ([16391bec](https://github.com/orogene/orogene/commit/16391becd01f9236fc22207faef38e94cfaa6e60))

<a name="0.3.31"></a>
## 0.3.31 (2023-09-28)

### Bug Fixes

* **dist:** temporarily allow-dirty and remove license stuff (#293) ([39685826](https://github.com/orogene/orogene/commit/3968582665fe35d1e01388df2030b88f1ab3c31c))

<a name="0.3.30"></a>
## 0.3.30 (2023-09-28)

### Features

* **docker:** add Dockerfile to compile orogene in a rust-debian-bookworm container (#288) ([10575974](https://github.com/orogene/orogene/commit/10575974ab7c2f6fcb2f862b281be14a9f7dbfdf))
* **auth:** add registry authentication support ([0513019f](https://github.com/orogene/orogene/commit/0513019f87f6d047bbb38864baa193e3a3711569))
* **login:** add support for all auth types to `oro login`, including direct `--token` passing ([cf709946](https://github.com/orogene/orogene/commit/cf70994664938ad45cb26c58d6b7c636c75e5fe8))

### Bug Fixes

* **auth:** glue login and authentication stuff, refactor, and make it all work ([23cf1bc8](https://github.com/orogene/orogene/commit/23cf1bc8b03a0c8b4f46a634ac2faff195dfd050))
* **auth:** don't send auth to non-registry URLs ([e84a2576](https://github.com/orogene/orogene/commit/e84a25768d0328c407224416680bf09ac48490f2))
* **misc:** get rid of annoying default-features warnings ([84f45865](https://github.com/orogene/orogene/commit/84f45865267b61bb2f783b5cbc7cbccb475a2e46))

### Documentation

* **auth:** Add authentication/authorization docs ([7ee6a742](https://github.com/orogene/orogene/commit/7ee6a742b4e5b7005b9e48a9a8acc1a8e02bae56))

<a name="0.3.29"></a>
## 0.3.29 (2023-09-27)

### Features

* **wasm:** publish NPM packages for nassun and node-maintainer as part of release ([f4f2ff22](https://github.com/orogene/orogene/commit/f4f2ff22cd6fec4bb5594f924c0773b9d92effa6))

<a name="0.3.28"></a>
## 0.3.28 (2023-09-27)

### Features

* **proxy:** support proxy configuration (#283) ([87facfe4](https://github.com/orogene/orogene/commit/87facfe44a14a61e94ee50c0a5f64724b065bdd8))
* **login:** Add `oro login ` and `oro logout` commands (#290) ([39b3c6bd](https://github.com/orogene/orogene/commit/39b3c6bdfa5327276666ef86bf08cb02d3800a7d))
* **dist:** bump cargo dist and add homebrew and msi artifacts ([218392d0](https://github.com/orogene/orogene/commit/218392d061fd234effb9fd12b5ba52e0d84c05c9))
* **retry:** enable request retries on wasm, too ([1f25aa33](https://github.com/orogene/orogene/commit/1f25aa3383793e40e64d20939fb9702050b2ff24))
* **wasm:** nicer TS types ([64d20dcc](https://github.com/orogene/orogene/commit/64d20dcc2361be9bc91894e1686e678ca827c836))

### Bug Fixes

* **proxy:** thread proxy settings through and tweak a few things along the way ([7d9b476e](https://github.com/orogene/orogene/commit/7d9b476ed2f732485b9f25655ed0598f0c2110aa))
* **site:** modernize oranda config ([427f672f](https://github.com/orogene/orogene/commit/427f672f8ffa3da699c42612ea642eb824587de3))
* **wasm:** get things working on wasm again! ([ec9e6e36](https://github.com/orogene/orogene/commit/ec9e6e36ce72719dacd15710ed7e3e3ae7498913))
* **lib:** fix readme/license fields in oro-npm-account ([0842d2bf](https://github.com/orogene/orogene/commit/0842d2bf00e825cebda3fe110a07451ffcea5c72))

<a name="0.3.27"></a>
## 0.3.27 (2023-05-21)

### Features

* **xxhash:** switch to xxhash, and always validate extraction (#274) ([dfdb378c](https://github.com/orogene/orogene/commit/dfdb378cbd5a7c7aa30012323c43770b933dcb14))

### Bug Fixes

* **reapply:** stop erroring on reapply if node_modules doesn't exist ([940333a9](https://github.com/orogene/orogene/commit/940333a96f865905b83debc5ea90704652c600ae))
* **node-maintainer:** return a more useful error message when symlink/junction creation both fail on Windows (#270) ([e254c393](https://github.com/orogene/orogene/commit/e254c39367bc662209bfb4c4a3b165d2ba849d0b))

<a name="0.3.26"></a>
## 0.3.26 (2023-05-19)

### Features

* **scripts:** run lifecycle scripts in correct dependency order (#263) ([00979ae8](https://github.com/orogene/orogene/commit/00979ae8af581ffeb37786f7da71b7a80022f826))
* **error:** Add extra context to all std::io::Error messages (#268) ([8749a526](https://github.com/orogene/orogene/commit/8749a5262d0b0412e0fbc5f3d8b8d2c4f179c0a3))

### Bug Fixes

* **resolver:** `PackageResolution::satisfies()` should use the spec target ([4f0fbba7](https://github.com/orogene/orogene/commit/4f0fbba75fe04f89383c23a39c354a67fdca1f00))
* **node-maintainer:** stop failing when root package entry in lockfile is missing package name ([b7ac680b](https://github.com/orogene/orogene/commit/b7ac680b161003cc67683597ce65d28119803fc6))

<a name="0.3.25"></a>
## 0.3.25 (2023-05-10)

### Bug Fixes

* **fmt:** cargo fmt --all ([887c2576](https://github.com/orogene/orogene/commit/887c2576ed5b771d7c863384b980ecaa21abc361))
* **telemetry:** lower telemetry sample rate ([ffbd6a88](https://github.com/orogene/orogene/commit/ffbd6a88fbc395ee9a8327b0b2fa09b8619425ab))

<a name="0.3.24"></a>
## 0.3.24 (2023-05-10)

### Features

* **optional:** ignore install script failures for truly optional dependencies ([09fcc77d](https://github.com/orogene/orogene/commit/09fcc77dc1ce0c3c0ce29c376c8cf53fe600a576))
* **credentials:** add support for parsing credentials (for later consumption) ([59f0a11c](https://github.com/orogene/orogene/commit/59f0a11c0b0e3639359780767b868f6f3c036df1))
* **telemetry:** implement opt-in crash and usage telemetry (#262) ([8d5e1f59](https://github.com/orogene/orogene/commit/8d5e1f591aedc0bbc2691b4e65b4652147f236d6))

### Bug Fixes

* **apply:** only update package.json on apply success ([1e775ad0](https://github.com/orogene/orogene/commit/1e775ad006a265b7162fbac6ff4f11de039f393c))
* **wasm:** missed a spot again with wasm ([c4fcb328](https://github.com/orogene/orogene/commit/c4fcb328e2cd92c2efd391c2ae72a271c126d93c))
* **wasm:** again ([b745c5bb](https://github.com/orogene/orogene/commit/b745c5bb76ae238e65bfac8238485d269e96253e))
* **docs:** add --credentials to command docs ([24b60124](https://github.com/orogene/orogene/commit/24b60124399c0e6b36da4efe156dad3f3b85d4b3))
* **debug-log:** stop printing ansi codes to debug log ([3c1cd69f](https://github.com/orogene/orogene/commit/3c1cd69fd8c3dab6cebea45162dab153751c12b4))
* **bin:** semi-preserve file modes and make binaries executable (#259) ([f01ed09d](https://github.com/orogene/orogene/commit/f01ed09d4b24db42e3901feebfc5bbc2dc0f4254))
* **apply:** fix regression where only hoisted linker was used ([78f41202](https://github.com/orogene/orogene/commit/78f4120254fe21bf788accc60bf35dc3c054628d))
* **misc:** fix a couple of straggler warnings ([594081f4](https://github.com/orogene/orogene/commit/594081f40db64624cd0aacf1c165118c6ce14b42))

### Documentation

* **readme:** improvements to README ([7b63b073](https://github.com/orogene/orogene/commit/7b63b073f3d68135133686c5defe941c8e5637a3))

<a name="0.3.23"></a>
## 0.3.23 (2023-04-18)

### Bug Fixes

* **docs:** update docs after `--lockfile` addition ([e1caff41](https://github.com/orogene/orogene/commit/e1caff41ca41ccef5c993d7ea37b3b9eed26d86d))
* **ping:** fix emoji spacing ([90038383](https://github.com/orogene/orogene/commit/900383837ab3697dd576f9e078882bf672296e3e))
* **nassun:** try and get rustdoc to show NassunError at the toplevel to play nicer with miette ([e5638712](https://github.com/orogene/orogene/commit/e5638712bdf9512403102fbda0353e9b171d7a23))
* **error:** export orogene cmd error enum ([b37e1167](https://github.com/orogene/orogene/commit/b37e1167192aecfbef95556107f75df4dd7dd07b))
* **wasm:** get node-maintainer working on wasm again ([b1f6bc82](https://github.com/orogene/orogene/commit/b1f6bc829dcd7690a238c4d2365f2d83277e3e97))

### Miscellaneous Tasks

* **deps:** bump miette to 5.8.0 ([e58c256e](https://github.com/orogene/orogene/commit/e58c256e36e0160f1874260f3fb71516463b4372))
* **deps:** bump supports-hyperlinks for more terminal support ([be5712e2](https://github.com/orogene/orogene/commit/be5712e2957db18a11267dc56e7dba099bf3683f))


<a name="0.3.22"></a>
## 0.3.22 (2023-04-18)

no-op due to release failure. Refer to 0.3.2 for actual release details.

<a name="0.3.21"></a>
## 0.3.21 (2023-04-18)

### Bug Fixes

* **release:** update cargo-dist release thing properly ([4a204a51](https://github.com/orogene/orogene/commit/4a204a5148122674fc719677cc848e6bbba01f53))

<a name="0.3.20"></a>
## 0.3.20 (2023-04-18)

### Features

* **reapply:** add reapply command and refactor apply to be reusable (#237) ([bf6b1504](https://github.com/orogene/orogene/commit/bf6b150462e40dc2d0a7f16ecf49130a336e67e3))
* **json:** add module for format-preserving JSON manipulation ([3fa23e46](https://github.com/orogene/orogene/commit/3fa23e4602ffc4a8f7038f48b9c7bb40994d94cb))
* **add:** add `oro add` command for adding new deps ([7ed9f777](https://github.com/orogene/orogene/commit/7ed9f777e886163ea9e02fc7dc011980d2c61036))
* **rm:** add oro rm command ([ec301bbf](https://github.com/orogene/orogene/commit/ec301bbfbce36c562fbd00419b12261aed5f1e96))
* **git:** support specifying semver ranges for git dependencies (#217) ([56e05f5b](https://github.com/orogene/orogene/commit/56e05f5b1417024ac65520d68e261f49ea70ab19))
* **locked:** add --locked mode support ([ffce208f](https://github.com/orogene/orogene/commit/ffce208fd1edff3f1e62093f19e567662ce8e1df))
* **docs:** write up a section about adding dependencies + specifiers ([e66936c4](https://github.com/orogene/orogene/commit/e66936c47ad6fd7b4f0b308d0b3cbece63386293))
* **errors:** better docs and url(docsrs) for all errors ([0629a080](https://github.com/orogene/orogene/commit/0629a08076c800f60789fe8f9feb60d176f5c432))

### Bug Fixes

* **metadata:** add support for boolean deserialization for `deprecated` tag of version metadata (#235) ([0505793a](https://github.com/orogene/orogene/commit/0505793ad92d3b9638034eee1943af46f396e9d5))
* **common:** handle deprecation booleans in registry response (#246) ([8ae12196](https://github.com/orogene/orogene/commit/8ae121963b91fd75c347102621f9c86b2db9fbce))
* **resolve:** stop lockfile from clobbering dependencies (#247) ([b3af2ddd](https://github.com/orogene/orogene/commit/b3af2ddd45335b7d937efa54f1a7110e0d3576cd))
* **log:** log the 'debug log way written' message as WARN ([dbcbe9cc](https://github.com/orogene/orogene/commit/dbcbe9cc12c471543d3cf84e496a7fb4400651c8))
* **rm:** warn/error when something other than a package name is provided ([26b01328](https://github.com/orogene/orogene/commit/26b0132822f82e92ce5362c3a30e4862e67bfe13))
* **error:** fix PackageSpecError docs and help not printing ([f8e8e27a](https://github.com/orogene/orogene/commit/f8e8e27a4205d2fbfdfd2a33123df27c90943d88))
* **config:** apply config options from subcommands, too ([3c31caac](https://github.com/orogene/orogene/commit/3c31caaca2c22267e7659848a8987297dcd51d61))
* **rm:** more random rm fixes ([1c733452](https://github.com/orogene/orogene/commit/1c7334522a00405a25f117155bb52d50d93d5d9f))

### Miscellaneous Tasks

* **deps:** bump h2 from 0.3.16 to 0.3.17 (#242) ([8123013f](https://github.com/orogene/orogene/commit/8123013f6c69464ef510a09aa09d9ac33e2db8a8))

<a name="0.3.19"></a>
## 0.3.19 (2023-04-10)

### Features

* **debug:** Include errors in debug log when possible ([9551239e](https://github.com/orogene/orogene/commit/9551239ee634244dc2fbef86627696614726c0ac))
* **isolated:** add support for isolated dependency installation (#226) ([9da2e1e7](https://github.com/orogene/orogene/commit/9da2e1e7231054437fbde2c7f8eaace9c1b67897))
* **apply:** rename `restore` to `apply` ([82f7b623](https://github.com/orogene/orogene/commit/82f7b623b3d657a5e02757c90567acefa9521481))

### Bug Fixes

* **binlink:** properly normalize non-object bin names ([aea368e9](https://github.com/orogene/orogene/commit/aea368e92c5d336b4c70d8f6506fb8fcc22b3273))
* **resolve:** use PackageSpec::target() for recursive alias support ([bd884703](https://github.com/orogene/orogene/commit/bd884703dafb32912883a8823a70a64d3fa71ae0))
* **ci:** how did this even happen ([d77a915e](https://github.com/orogene/orogene/commit/d77a915e4c1d64503ed2b325b93501d7259d4bc0))

<a name="0.3.18"></a>
## 0.3.18 (2023-04-07)

### Features

* **config:** load options from env, too ([ccfa812b](https://github.com/orogene/orogene/commit/ccfa812b54d6f22089369773e2780c48b927d670))
* **docs:** More detailed configuration docs ([6165e808](https://github.com/orogene/orogene/commit/6165e8083bbbac7ce864ccf329b9adf61be9bda4))
* **ping:** emojify ping ([a6627657](https://github.com/orogene/orogene/commit/a66276570dec663c1bec43b681e72e232f10d2b5))
* **config:** read config file options from `options` node ([41b281ee](https://github.com/orogene/orogene/commit/41b281eecc4a66275bc0abdfdc1e0afadad195ff))
* **docs:** show command aliases in docs ([0dc70672](https://github.com/orogene/orogene/commit/0dc706725f534c48daaae648323f25145b1b34c6))
* **config:** add support for nested arrays and maps to kdl configs ([f5e71d0c](https://github.com/orogene/orogene/commit/f5e71d0c9f83d1a9ef2376bc402d1d78d6e15fbc))
* **config:** support for specifying scope registries ([49c2190e](https://github.com/orogene/orogene/commit/49c2190e7a40a4b7ca19ac7ddc342f647b96ce63))
* **config:** support for arrays, too ([ef46e5aa](https://github.com/orogene/orogene/commit/ef46e5aa84a3c3ce389a8f45d8441acf09d0c719))
* **config:** overhaul how scoped registries are provided ([31c3ae74](https://github.com/orogene/orogene/commit/31c3ae742988dd0ff0235156ae7b302f3e6a4421))

### Bug Fixes

* **view:** remove stray `dbg!` ([03f7a7dd](https://github.com/orogene/orogene/commit/03f7a7ddcf39b6e7afd3bdb11c645dbdd147ef25))

<a name="0.3.17"></a>
## 0.3.17 (2023-04-03)

### Bug Fixes

* **log:** go back to not displaying targets for terminal logs ([5cb06a75](https://github.com/orogene/orogene/commit/5cb06a75613c670fd1bc05bc8226a3223de5fdb7))

### Features

* **config:** add support for `--no-<conf>` and overhaul config fallbacks ([868a42b5](https://github.com/orogene/orogene/commit/868a42b5bb30654b150f3b2dfec3438c8db8b301))
* **config:** config files are now kdl-based ([09e81bd9](https://github.com/orogene/orogene/commit/09e81bd9260b79f1e7a1d94aef4766d29f0f5582))


<a name="0.3.16"></a>
## 0.3.16 (2023-04-01)

### Bug Fixes

* **deps:** bump deps/miette/etc ([b987939e](https://github.com/orogene/orogene/commit/b987939ea78902bf63f14f8d6dbd109d20872d35))
* **node-maintainer:** improve api docs and remove some undesirable APIs in node-maintainer ([2dea36c0](https://github.com/orogene/orogene/commit/2dea36c05eeacd5467108e7b7d6ae629723188a0))
* **docs:** update CLI docs ([bd0d7fda](https://github.com/orogene/orogene/commit/bd0d7fda76a52d77d994c628fecd8b14c515448b))
* **ci:** add rust-versions and re-enable minimal version checks ([b509cc20](https://github.com/orogene/orogene/commit/b509cc2042884ac373f57d25705a615772ded3d8))

### Features

* **wasm:** more wasm fixes + expose iteration functionality (#218) ([03dce2e9](https://github.com/orogene/orogene/commit/03dce2e92a7f05dd0e0700286f343a9b14e718d7))
* **restore:** allow configuring script concurrency and whether to write lockfiles ([a681e64a](https://github.com/orogene/orogene/commit/a681e64a6cb8d38a9e7396b7f5ec5cc4326906dc))
* **config:** have OroLayerConfig obey `#[arg(skip)]` ([fc5f53ae](https://github.com/orogene/orogene/commit/fc5f53ae7eed8f78dd08dfb77f4833d97d92612d))


<a name="0.3.15"></a>
## 0.3.15 (2023-03-31)

### Bug Fixes

* **shim:** fix .cmd shim targets ([020d96cf](https://github.com/orogene/orogene/commit/020d96cf9537f56982c53cfa9768691edf6207d3))
* **manifest:** unshadow the actual output of BuildManifest::normalize (#216) ([e5c8d4bb](https://github.com/orogene/orogene/commit/e5c8d4bbb9ee08d1e75f032550ac5eed4522edff))

### Features

* **manifests:** add a from_manifest method to BuildManifest and do some drive-by docs work (#213) ([2e9c4f51](https://github.com/orogene/orogene/commit/2e9c4f51008456e34dda7d3be3465702a433180e))
* **scripts:** run lifecycle scripts (#209) ([48392c3e](https://github.com/orogene/orogene/commit/48392c3e62cdf244a960a3fb1e83cda0f320f198))


<a name="0.3.14"></a>
## 0.3.14 (2023-03-26)

### Features

* **build:** link bins (#212) ([e8ed3ff5](https://github.com/orogene/orogene/commit/e8ed3ff5a83d56dcb347f2734ac63738ae15bd91))

<a name="0.3.13"></a>
## 0.3.13 (2023-03-25)

### Security

* **deps:** bump openssl from 0.10.45 to 0.10.48 (#211) ([e87243c2](https://github.com/orogene/orogene/commit/e87243c2e22bbbffd8e3d30782c48fe6994c9416))


<a name="0.3.12"></a>
## 0.3.12 (2023-03-25)

### Features

* **log:** improved logging/output by changing levels and formatting ([e1cafd0c](https://github.com/orogene/orogene/commit/e1cafd0cce46d989f1ee913a1d18f41c8097162d))
* **gitinfo:** add a FromStr impl for GitInfo ([308d9ab7](https://github.com/orogene/orogene/commit/308d9ab7b08c9d143efe6fd3466b93b67bb58c40))

### Bug Fixes

* **tests:** Nassun baseline tests (#196) ([58e76853](https://github.com/orogene/orogene/commit/58e768535e86f6b9aef81d92fa8b69a95f0aaf69))

<a name="0.3.11"></a>
## 0.3.11 (2023-03-19)

Most of this release was docs (which are available through [the Orogene
site!](https://orogene.dev/book)), but there's some emoji-related stuff fixed,
too that might be handy.

### Features

* **msg:** fasterthanlime is basically the lemonodor of rust, no? ([fcc5a256](https://github.com/orogene/orogene/commit/fcc5a2565622317aad5ce4c669813cbeef44a1cf))
* **docs:** initial mdbook setup and hookup to oranda (#205) ([b66a66e0](https://github.com/orogene/orogene/commit/b66a66e0567fdb4993a6c03b848ef4d9ab0d4f45))
* **emoji:** add global flag to disable emoji display ([bafbe802](https://github.com/orogene/orogene/commit/bafbe802f3c3014525c79a39a182f0e89b8c6487))


### Bug Fixes

* **emoji:** don't print emoji when unicode isn't supported ([e8a8af79](https://github.com/orogene/orogene/commit/e8a8af791a9974c7eb29547700d336bbe37b47ce))
* **wasm:** missed a couple of wasm spots after recent changes ([0e4d8b03](https://github.com/orogene/orogene/commit/0e4d8b030724599172dfc1d3ce0437271fef8336))
* **git:** use once_cell instead of mutexes for git cache path ([5961dfbc](https://github.com/orogene/orogene/commit/5961dfbc03fc93fe458f98057af266b4f2ee240f))

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

