<template>
  <div id="app">
    <img alt="Vue logo" src="./assets/logo.png">
    <HelloWorld msg="Welcome to Your Vue.js + TypeScript App"/>
    <button @click="sayHelloBlocking()">Hello Blocking</button>
    <button @click="sayHelloAsync()">Hello Async</button>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import HelloWorld from './components/HelloWorld.vue';
import { invoke, promisified } from 'tauri/api/tauri'

interface AsyncEchoResponse {
  msg: string
}

export default defineComponent({
  name: 'App',
  components: {
    HelloWorld
  },
  methods: {
    sayHelloBlocking () {
      invoke({
        cmd: 'blockingEcho',
        msg: 'blocking hello from js!'
      })
    },
    sayHelloAsync () {
      promisified<AsyncEchoResponse>({
        cmd: 'asyncEcho',
        msg: 'async hello from js!'
      }).then(result => {
        alert(result.msg)
      }).catch(console.error)
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
