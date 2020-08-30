import { createApp } from 'vue';
import App from './App.vue';
import router from './router';

import { library } from '@fortawesome/fontawesome-svg-core';
import { faUserSecret } from '@fortawesome/free-solid-svg-icons';
// @ts-ignore expect error here since this is not a TS file. This is until the currently pending PR on that repo is accepted and merged.
import { FontAwesomeIcon } from './assets/temp-deps/vue-fontawesome';

library.add(faUserSecret);

const app = createApp(App);

/*
  register font-awesome-icon component globally.
  example usage:

  ```html
    <font-awesome-icon icon="user-secret" />
  ```
 */
app.component('font-awesome-icon', FontAwesomeIcon);
app.use(router);
app.mount('#app');
