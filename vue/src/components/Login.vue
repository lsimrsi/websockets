<script setup lang="ts">
  import { useNameStore, useSocketStore } from "../stores";
  import Input from "./Input.vue";

  const name = useNameStore();
  const socket = useSocketStore();

  function onSubmit(e: Event) {
    e.preventDefault();
    console.log("name", name.name);
    socket.sendMessage({ msg_type: "RegisterName", data: name.name });
  }
</script>

<template>
  <form class="m-auto" @submit="onSubmit">
    <Input label="Please enter your name:" id="name" v-model="name.name" />
  </form>
</template>
