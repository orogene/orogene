<template>
  <div id="app">
    <img alt="Vue logo" src="./assets/logo.png">
    <HelloWorld msg="Welcome to Your Vue.js + TypeScript App"/>
    <button @click="sayHello()">Hello</button>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import HelloWorld from './components/HelloWorld.vue';
import { invoke } from 'tauri/api/tauri'

export default defineComponent({
  name: 'App',
  components: {
    HelloWorld
  },
  methods: {
    sayHello () {
      console.log('hello!')
      invoke({
        cmd: 'myCustomCommand',
        argument: 'hello from js!'
      })
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
  margin-top: 60px;
}
</style>
