<script lang="ts">
  import type { ChatMessage } from "../interfaces";
  import { name } from "../stores";
  export let chatMsg: ChatMessage;

  let nameStyle = chatMsg.name === $name ? "text-blue-500" : "text-zinc-500";
</script>

<div class="flex flex-col mb-4">
  <span class={`font-bold ${nameStyle} min-w-[8rem]`}>
    {chatMsg.name}
  </span>
  <span class="border-l-2 pl-4 text-zinc-500">
    {chatMsg.message}
  </span>
</div>
