<script lang="ts">
  export let id: string = "";
  export let value: string = "";
  export let classes: string = "";
</script>

<input
  class={`border py-2 px-4 bg-slate-200 hover:bg-slate-100 active:bg-white text-slate-600 ${classes}`}
  type="button"
  {id}
  {value}
/>
