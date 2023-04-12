<script setup lang="ts">
  import type { ChatMessage } from "../interfaces";
  // import { useNameStore } from "../stores";

  defineProps<{
    chatMsg: ChatMessage
  }>()

  // const name = useNameStore();

  // let nameStyle = chatMsg.name === $name ? "text-blue-500" : "text-zinc-500";
</script>

<template>
  <div class="flex flex-col mb-4">
    <span class="`font-bold min-w-[8rem]`">
      {{chatMsg.name}}
    </span>
    <span class="border-l-2 pl-4 text-zinc-500">
      {{chatMsg.message}}
    </span>
  </div>
</template>
