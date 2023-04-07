<script lang="ts">
  import { toasts } from "../stores";
  import Toast from "./toast.svelte";
</script>

<div data-testid="toasts" class="absolute top-4 right-4 pointer-events-none">
  {#each $toasts as toast (toast.uuid)}
    <Toast {toast} />
  {/each}
</div>
