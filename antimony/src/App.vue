<template>
  <div id="app">
    <div id="nav">
      <router-link to="/">Home</router-link> |
      <router-link to="/about">About</router-link>
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
          cmd: "ping",
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
  color: #2c3e50;
}

#nav {
  padding: 30px;
}

#nav a {
  font-weight: bold;
  color: #2c3e50;
}

#nav a.router-link-exact-active {
  color: #42b983;
}
</style>
