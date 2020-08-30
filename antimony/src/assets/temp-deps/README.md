## Temporary dependencies

As Vue3 is still in beta, some libraries are stil not compatible, or may have pending fixes.
But we don't necesarily want to wait until the fix comes out.
If the code exists but the PR is not merged or the new version not published on npm,
we can clone the repo, and build a version with the fix with we need.

This folder is for those files, which will become obsolete as soon as the relevant fixes are published on NPM.

_We will not maintain custom builds_

This folder is _NOT_ for custom builds of external libraries that would require long term maintenance.

Here are the libraries we are including so far:

#### vue-router

The release code for [vue-router@4.0.0-beta.7](https://github.com/vuejs/vue-router-next) currently includes use of spread and rest operator, as well as a dangling comma.
This breaks the microsfot edge webview tauri uses on windows.
This has been [fixed](https://github.com/vuejs/vue-router-next/issues/304#issuecomment-679161877) on the main branch of vue-router-next, so it should be out on the next minor release.

#### vue-fontawesome

vue-fontawesome is not yet Vue 3 compatible, but a working PR exists [here](https://github.com/FortAwesome/vue-fontawesome/pull/246).
