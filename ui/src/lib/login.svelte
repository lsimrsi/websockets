<script lang="ts">
  import Input from "./input.svelte";

  function onSubmit(e: Event) {
    e.preventDefault();
  }
</script>

<form on:submit={onSubmit}>
  <Input label="Please enter your name:" id="name" />
</form>
