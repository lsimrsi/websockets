<script lang="ts">
  import type { ChatMessage } from "../interfaces";
  export let chatMsg: ChatMessage;
</script>

<div class="flex">
  <span class="p-2 mb-1 bg-zinc-100 text-zinc-600">
    {chatMsg.name}
  </span>
  <span class="p-2">
    {chatMsg.message}
  </span>
</div>
