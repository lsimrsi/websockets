<script lang="ts">
  import type { ChatMessage } from "../interfaces";
  export let chatMsg: ChatMessage;
</script>

<div class="flex flex-col mb-4">
  <span class="font-bold text-zinc-600 min-w-[8rem]">
    {chatMsg.name}
  </span>
  <span class="border-l-2 pl-4 text-zinc-500">
    {chatMsg.message}
  </span>
</div>
