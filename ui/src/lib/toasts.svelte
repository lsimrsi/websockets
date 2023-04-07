<script lang="ts">
  import { toasts } from "../stores";
  import Toast from "./toast.svelte";
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { cubicInOut, cubicOut } from "svelte/easing";
</script>

<div data-testid="toasts" class="absolute top-4 right-4 pointer-events-none">
  {#each $toasts as toast (toast.uuid)}
    <div
      in:fly={{ y: 50, duration: 200, easing: cubicOut }}
      out:fly={{ y: -50, duration: 600, easing: cubicInOut }}
      animate:flip={{ duration: 200 }}
      class={`flex right-0 w-96 transition-[top] rounded-bl-lg rounded-tr-lg mb-2 border shadow bg-white`}
    >
      <Toast {toast} />
    </div>
  {/each}
</div>
