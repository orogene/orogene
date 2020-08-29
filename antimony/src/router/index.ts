// use out own vue-router for now, until the edge fix goes live on npm
import {
  createRouter,
  createWebHashHistory,
  RouteRecordRaw
  // @ts-ignore expect error here since this is not a TS file. This is until the next router version
} from '@/assets/temp-deps/vue-router.esm.js';
// import { createRouter, createWebHashHistory, RouteRecordRaw } from 'vue-router';

import Home from '../views/Home.vue';

const routes: Array<RouteRecordRaw> = [
  {
    path: '/',
    name: 'Home',
    component: Home
  },
  {
    path: '/about',
    name: 'About',
    // route level code-splitting
    // this generates a separate chunk (about.[hash].js) for this route
    // which is lazy-loaded when the route is visited.
    component: () =>
      import(/* webpackChunkName: "about" */ '../views/About.vue')
  }
];

const router = createRouter({
  history: createWebHashHistory(),
  routes
});

export default router;
