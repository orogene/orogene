// use out own vue-router for now, until the edge fix goes live on npm
import {
  createRouter,
  createWebHashHistory,
  RouteRecordRaw
  // @ts-ignore expect error here since this is not a TS file. This is until the next router version
} from '@/assets/vue-router.esm.js';
// import { createRouter, createWebHashHistory, RouteRecordRaw } from 'vue-router';

import Browse from '../views/Browse.vue';

const routes: Array<RouteRecordRaw> = [
  {
    path: '/',
    name: 'Browse',
    component: Browse
  },
  {
    path: '/installed',
    name: 'Installed',
    component: () => import('../views/Installed.vue')
  },
  {
    path: '/updates',
    name: 'Updates',
    component: () => import('../views/Updates.vue')
  }
];

const router = createRouter({
  history: createWebHashHistory(),
  routes
});

export default router;
