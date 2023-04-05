<script lang="ts">
  export let label: string;
  export let id: string;
  let value = "";
</script>

<div>
  <label for={id}>{label}</label>
  <input {id} bind:value />
</div>
