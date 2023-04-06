<script lang="ts">
  export let label: string = "";
  export let id: string = "";
  export let value: string = "";
  export let classes: string = "";
</script>

<div class="flex flex-col">
  <label class="text-slate-600" for={id}>{label}</label>
  <input class={`border p-2 ${classes}`} {id} bind:value />
</div>
