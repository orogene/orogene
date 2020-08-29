<template>
  <div id="app">
    <div id="nav">
      <router-link to="/">browse</router-link>
      <router-link to="/installed">installed</router-link>
      <router-link to="/updates">updates</router-link>
    </div>
    <router-view />
    <button @click="ping()">Ping registry</button>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { promisified } from 'tauri/api/tauri';

interface PingResponse {
  body: {
    registry: string;
    time: number;
    details: object;
  };
}

export default defineComponent({
  name: 'App',
  methods: {
    async ping() {
      try {
        let { body } = await promisified<PingResponse>({
          cmd: 'ping',
          args: null
        });
        alert(JSON.stringify(body, null, 2));
      } catch (e) {
        console.error(e);
      }
    }
  }
});
</script>

<style>
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: #bdbdbd;
  background-color: #434343;
  height: 36rem;
}

#nav {
  padding: 30px;
  font-size: 1.5rem;
}

#nav a {
  margin: 1rem;
  padding: 0.25rem;
  font-weight: normal;
  color: #bdbdbd;
  text-decoration: none;
}

#nav a.router-link-exact-active {
  color: #7ae1c5;
  border-bottom: 2px solid;
  border-color: #7ae1c5;
}
</style>
