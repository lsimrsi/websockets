<script setup lang="ts">
  import { useNameStore, useSocketStore } from "../stores";
  // import Input from "./input.svelte";

  const name = useNameStore();
  const socket = useSocketStore();

  function onSubmit(e: Event) {
    e.preventDefault();
    console.log("name", name.name);
    socket.sendMessage({ msg_type: "RegisterName", data: name.name });
  }
</script>

<template>
  <form class="m-auto" :submit={onSubmit}>
    <label>Please enter your name</label>
    <input v-model="name.name"/>
  </form>
</template>
