<script lang="ts">
  export let label: string = "";
  export let id: string = "";
  export let value: string = "";
</script>

<div class="flex flex-col">
  <label for={id}>{label}</label>
  <input class="border" {id} bind:value />
</div>
