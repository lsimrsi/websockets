<script lang="ts">
  import { toasts } from "../stores";
  import Toast from "./toast.svelte";

  import { quintOut } from "svelte/easing";
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";

  // const [send, receive] = crossfade({
  //   fallback(node, params) {
  //     const style = getComputedStyle(node);
  //     const transform = style.transform === "none" ? "" : style.transform;

  //     return {
  //       duration: 400,
  //       easing: quintOut,
  //       css: (t) => {
  //         let y = (1 - t) * 50;
  //         return `
  // 				transform: ${transform} translateY(${y}px);
  // 				opacity: ${t}
  // 			`;
  //       },
  //     };
  //   },
  // });
</script>

<div data-testid="toasts" class="absolute top-4 right-4 pointer-events-none">
  {#each $toasts as toast (toast.uuid)}
    <div
      in:fly={{ y: 50, duration: 200 }}
      out:fly={{ y: -50, duration: 600 }}
      animate:flip={{ duration: 200 }}
      class={`flex right-0 w-96 transition-[top] rounded-bl-lg rounded-tr-lg mb-2 border shadow bg-white`}
    >
      <Toast {toast} />
    </div>
  {/each}
</div>
